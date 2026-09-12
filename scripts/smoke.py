#!/usr/bin/env python3
"""Offline, isolated checks of the real host and cartridge protocol entry points."""
import contextlib
import http.client
import json
import os
from pathlib import Path
import secrets
import re
import signal
import socket
import subprocess
import sys
import tempfile
import time

from workspace import ROOT, target_directory


@contextlib.contextmanager
def child(command, cwd, **kwargs):
    process = subprocess.Popen(command, cwd=cwd, start_new_session=True, **kwargs)
    try:
        yield process
    finally:
        # These processes belong to this smoke test, including any SDK descendants.
        for sig in (signal.SIGTERM, signal.SIGKILL):
            try:
                os.killpg(process.pid, sig)
            except ProcessLookupError:
                break
            try:
                process.wait(timeout=3)
            except subprocess.TimeoutExpired:
                continue
            # The parent can exit before a descendant; finish only our process group.
            try:
                os.killpg(process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            break
        process.wait(timeout=3)


def command(profile):
    executable = target_directory() / "debug/cartridge"
    if not executable.is_file():
        raise RuntimeError("Build first: just build")
    builtin = profile / "builtin"
    return [str(executable), "--dir", str(builtin if builtin.is_dir() else ROOT / "builtin"), "--profile", str(profile)]


def profile(root, name, port=0):
    directory = root / name
    directory.mkdir()
    init = (ROOT / ".cartridge" / name / "init.lua").read_text()
    (directory / "init.lua").write_text(init)
    builtin = directory / "builtin"
    builtin.mkdir()
    for module in re.findall(r'path\s*=\s*"([^"\n]+)"', init):
        if name == "proxy" and module == "router":
            fixture = builtin / module
            fixture.mkdir()
            (fixture / "cartridge.json").write_text(json.dumps({"name":"router","entry":"init.lua"}))
            (fixture / "init.lua").write_text('return {provide={"router"},apply=function(ctx) ctx:provide("router",function() return {data={}} end) end}')
        else:
            (builtin / module).symlink_to((ROOT / "builtin" / module).resolve())
    # Isolated state and no configured model providers. No live profile data is changed.
    (directory / "config.lua").write_text('''return {
        sessions={dir="sessions"}, memory={dir="memory", reason={url=""},
            tick={interval_secs=0}, queue={enabled=false}},
        router={listen={"127.0.0.1:0"},config_dir="credentials",data_dir="router"},
        proxy={listen="127.0.0.1:%d",cwd=".",key_env="CARTRIDGE_PROXY_KEY"},
        mcp={cwd="."}, policy={default="ask"}, harness={max_bytes=262144},
        gitfs={store_dir="gitfs"}
    }''' % port)
    return directory


def mcp(root):
    requests = [
        {"jsonrpc":"2.0","id":1,"method":"initialize","params":{
            "protocolVersion":"2025-06-18","capabilities":{},
            "clientInfo":{"name":"cartridge-development-smoke","version":"1"}}},
        {"jsonrpc":"2.0","method":"notifications/initialized"},
        {"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}
    ]
    with child(command(profile(root, "mcp")) + ["mcp"], root,
               stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True) as process:
        out, err = process.communicate("".join(json.dumps(r)+"\n" for r in requests), timeout=30)
        if process.returncode:
            raise RuntimeError(err)
    replies = {r["id"]: r for r in map(json.loads, out.splitlines()) if "id" in r}
    assert "error" not in replies[1] and "result" in replies[1], replies
    tools = replies[2]["result"]["tools"]
    assert tools and all("name" in tool and "inputSchema" in tool for tool in tools), replies
    print(f"MCP initialize and tools/list passed ({len(tools)} tools)")


def proxy(root):
    with socket.socket() as reservation:
        reservation.bind(("127.0.0.1", 0))
        port = reservation.getsockname()[1]
    key = secrets.token_hex(32)
    with tempfile.TemporaryFile(mode="w+") as log:
        with child(command(profile(root, "proxy", port)) + ["daemon"], root,
                   stdout=log, stderr=log, env={**os.environ, "CARTRIDGE_PROXY_KEY":key}) as process:
            deadline = time.monotonic() + 30
            while True:
                connection = http.client.HTTPConnection("127.0.0.1", port, timeout=1)
                try:
                    connection.request("GET", "/development-probe")
                    response = connection.getresponse()
                    assert response.status == 401, response.status
                    response.read()
                    break
                except (OSError, http.client.HTTPException):
                    if process.poll() is not None or time.monotonic() >= deadline:
                        log.seek(0)
                        raise RuntimeError("Proxy did not become ready:\n" + log.read())
                    time.sleep(0.05)
                finally:
                    connection.close()
            connection = http.client.HTTPConnection("127.0.0.1", port, timeout=3)
            try:
                connection.request("GET", "/development-probe", headers={"Authorization":"Bearer "+key})
                response = connection.getresponse()
                assert response.status == 404, response.status
                response.read()
            finally:
                connection.close()
    print("Proxy HTTP listener and authentication passed (no model request sent)")


def policy(root):
    p = root / "policy"
    p.mkdir()
    (p / "init.lua").write_text('return {{id="policy",path="policy"}}\n')
    (p / "config.lua").write_text('return {policy={default="ask"}}\n')
    (p / "builtin").mkdir()
    (p / "builtin/policy").symlink_to((ROOT / "builtin/policy").resolve())
    cases = [("read", "allow"), ("write", "ask"), ("unknown-tool", "ask")]
    for tool, decision in cases:
        request = {"tool":tool,"input":{},"context":{}}
        result = subprocess.run(command(p)+["run","policy",json.dumps(request)], cwd=root,
                                check=True, capture_output=True, text=True, timeout=15)
        assert json.loads(result.stdout)["decision"] == decision, result.stdout
    result = subprocess.run(command(p)+["run","policy","null"], cwd=root,
                            check=True, capture_output=True, text=True, timeout=15)
    assert json.loads(result.stdout)["decision"] == "deny", result.stdout
    print("Policy allow/ask/deny protocol checks passed")


if __name__ == "__main__":
    selected = sys.argv[1] if len(sys.argv) > 1 else "all"
    checks = {"policy":policy,"proxy":proxy,"mcp":mcp}
    try:
        if selected != "all" and selected not in checks:
            raise RuntimeError("Choose all, policy, proxy, or mcp")
        for name, check in checks.items():
            if selected in ("all", name):
                with tempfile.TemporaryDirectory(prefix=f"cartridge-{name}-smoke-") as directory:
                    check(Path(directory))
    except (RuntimeError, AssertionError, KeyError, ValueError, subprocess.SubprocessError) as error:
        print(f"Smoke failed: {error}", file=sys.stderr)
        if getattr(error, "stderr", None):
            print(error.stderr, file=sys.stderr)
        sys.exit(1)
