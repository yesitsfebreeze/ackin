#!/usr/bin/env python3
"""Repeatable local runtime evidence; no external Python dependencies.

python3 .pearde/quality-tools/measure.py CHECKOUT OUTPUT [--checks]
The private Cargo target remains in /tmp so later rounds can reuse compilation.
"""
import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import platform
import re
import signal
import socket
import statistics
import subprocess
import tempfile
import time


def rust_code_mask(source):
    """Blank comments/literals, retaining offsets/newlines for item boundaries."""
    masked = list(source)
    raw_string = re.compile(r'(?:br|cr|r)(#*)"')
    character = re.compile(r"'(?:\\(?:u\{[0-9A-Fa-f_]+\}|x[0-9A-Fa-f]{2}|.)|[^'\\\n])'")
    index = 0
    while index < len(source):
        end = None
        if source.startswith("//", index):
            end = source.find("\n", index)
            end = len(source) if end < 0 else end
        elif source.startswith("/*", index):
            end, depth = index + 2, 1
            while end < len(source) and depth:
                if source.startswith("/*", end):
                    depth, end = depth + 1, end + 2
                elif source.startswith("*/", end):
                    depth, end = depth - 1, end + 2
                else:
                    end += 1
        else:
            raw = raw_string.match(source, index)
            if raw:
                closing = '"' + raw[1]
                position = source.find(closing, raw.end())
                end = len(source) if position < 0 else position + len(closing)
            elif source[index] == '"':
                end = index + 1
                while end < len(source):
                    if source[end] == "\\":
                        end += 2
                    elif source[end] == '"':
                        end += 1
                        break
                    else:
                        end += 1
            elif source[index] == "'":
                # A character literal, not a lifetime such as 'static or 'a.
                char = character.match(source, index)
                if char:
                    end = char.end()
        if end is None:
            index += 1
        else:
            for at in range(index, min(end, len(source))):
                if source[at] != "\n":
                    masked[at] = " "
            index = end
    return "".join(masked)


def inline_test_ranges(source):
    """Inclusive 1-based ranges for exact #[cfg(test)] items/modules."""
    code = rust_code_mask(source)
    ranges, consumed = [], 0
    for match in re.finditer(r"#\s*\[\s*cfg\s*\(\s*test\s*\)\s*\]", code):
        if match.start() < consumed:
            continue
        stack = []
        body = False
        for end in range(match.end(), len(code)):
            token = code[end]
            if token in "([{":
                if token == "{" and not stack:
                    body = True
                stack.append(token)
            elif token in ")]}":
                if not stack:
                    raise ValueError("unbalanced Rust item after #[cfg(test)]")
                stack.pop()
                if body and not stack:
                    break
            elif token == ";" and not stack:
                break
        else:
            raise ValueError("unterminated Rust item after #[cfg(test)]")
        consumed = end + 1
        ranges.append((source.count("\n", 0, match.start()) + 1,
                       source.count("\n", 0, consumed) + 1))
    return ranges


def source_line_counts(checkout):
    groups = {name: {"files": 0, "physical_lines": 0, "nonblank_lines": 0,
                     "by_file": {}} for name in ("production", "tests_and_fixtures")}
    inline, unresolved = {}, []
    for path in sorted((checkout / "src").rglob("*.rs")):
        relative = str(path.relative_to(checkout))
        source = path.read_text()
        lines = source.splitlines()
        ranges = inline_test_ranges(source)
        if ranges:
            inline[relative] = ranges
        test_lines = {line for start, end in ranges for line in range(start, end + 1)}
        # Keep unfamiliar conditional test syntax visible; do not guess cfg logic.
        for match in re.finditer(r"#\s*\[\s*cfg(?:_attr)?\b[^\]]*\]", rust_code_mask(source)):
            if re.search(r"\btest\b", match[0]) and not re.fullmatch(
                    r"#\s*\[\s*cfg\s*\(\s*test\s*\)\s*\]", match[0]):
                unresolved.append({"file": relative,
                    "line": source.count("\n", 0, match.start()) + 1,
                    "attribute": match[0]})
        is_test_file = "tests" in path.relative_to(checkout / "src").parts
        for name, group in groups.items():
            selected = [line for number, line in enumerate(lines, 1)
                        if (is_test_file or number in test_lines) == (name == "tests_and_fixtures")]
            if not selected:
                continue
            size = {"physical_lines": len(selected),
                    "nonblank_lines": sum(bool(line.strip()) for line in selected)}
            group["files"] += 1
            group["by_file"][relative] = size
            for field, count in size.items():
                group[field] += count
    return groups, inline, unresolved


def source_snapshot(checkout):
    """Re-enumerate input scope each time, including non-Rust test fixtures."""
    paths = set()
    for directory in (checkout / "src", checkout / ".cargo"):
        if directory.is_dir():
            paths.update(path for path in directory.rglob("*") if path.is_file())
    for name in ("Cargo.toml", "Cargo.lock", "build.rs", "rustfmt.toml", ".rustfmt.toml"):
        path = checkout / name
        if path.is_file():
            paths.add(path)
    return {str(path.relative_to(checkout)): hashlib.sha256(path.read_bytes()).hexdigest()
            for path in sorted(paths)}


def summary(values):
    ordered = sorted(values)
    return {
        "samples": len(values),
        "median_ms": statistics.median(values),
        "p95_ms": ordered[math.ceil(len(ordered) * .95) - 1],
        "min_ms": ordered[0],
        "max_ms": ordered[-1],
    }


class Connection:
    def __init__(self, path):
        self.sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self.sock.settimeout(10)
        try:
            self.sock.connect(str(path))
        except BaseException:
            self.sock.close()
            raise
        self.reader = self.sock.makefile("rb")
        self.counter = 0

    def send(self, key, payload):
        self.counter += 1
        request_id = self.counter
        frame = json.dumps({"call": key, "args": payload, "id": request_id},
                           separators=(",", ":")).encode() + b"\n"
        started = time.perf_counter_ns()
        self.sock.sendall(frame)
        return request_id, started

    def receive(self):
        while True:
            line = self.reader.readline()
            if not line:
                raise RuntimeError("daemon closed the benchmark socket")
            frame = json.loads(line)
            if "reply" in frame:
                return frame, time.perf_counter_ns()

    def batch(self, key, payload, count):
        # One persistent connection, bounded count of simultaneous requests.
        pending = dict(self.send(key, payload) for _ in range(count))
        timings = []
        for _ in range(count):
            reply, ended = self.receive()
            started = pending.pop(reply["reply"])
            if "error" in reply:
                raise RuntimeError(reply["error"])
            expected = payload if key == "lua" else {
                "data": payload, "meta": None, "absent": None}
            if reply.get("data") != expected:
                raise AssertionError("incorrect RPC response: " + repr(reply))
            timings.append((ended - started) / 1e6)
        return timings

    def close(self):
        self.reader.close()
        self.sock.close()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("checkout", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--target-dir", type=Path)
    parser.add_argument("--checks", action="store_true")
    parser.add_argument("--samples", type=int, default=3200)
    parser.add_argument("--warmups", type=int, default=320)
    parser.add_argument("--startup-samples", type=int, default=30)
    args = parser.parse_args()
    checkout, output = args.checkout.resolve(), args.output.resolve()
    if (output / "results.json").exists():
        parser.error("evidence already exists; choose a new output directory to preserve previous rounds")
    output.mkdir(parents=True, exist_ok=True)
    target = args.target_dir or Path(tempfile.mkdtemp(prefix="cartridge-quality-target-", dir="/tmp"))
    target = target.resolve()
    if not str(target).startswith(("/tmp/", "/private/tmp/")):
        parser.error("--target-dir must be isolated under /tmp")
    env = dict(os.environ, CARGO_TARGET_DIR=str(target))
    evidence = {"protocol": "local-rpc-v1", "checkout": str(checkout),
                "target_dir": str(target), "checks": {}, "workloads": {},
                "runtime_identity": {"package": "cartridge", "binary": "cartridge",
                    "lua_api": "cartridge.process", "profile_directory": ".cartridge",
                    "rename_from_baseline": "zirkle -> cartridge; workload unchanged"},
                "measurement_driver_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest()}

    def save():
        (output / "results.json").write_text(json.dumps(evidence, indent=2) + "\n")

    def run(name, command, timeout=1200):
        print("Running " + " ".join(command), flush=True)
        started = time.perf_counter()
        try:
            proc = subprocess.run(command, cwd=checkout, env=env, capture_output=True,
                                  text=True, timeout=timeout)
            code, stdout, stderr = proc.returncode, proc.stdout, proc.stderr
        except subprocess.TimeoutExpired as exc:
            code = "timeout"
            stdout = (exc.stdout or b"").decode(errors="replace")
            stderr = (exc.stderr or b"").decode(errors="replace")
        (output / (name + ".stdout.log")).write_text(stdout)
        (output / (name + ".stderr.log")).write_text(stderr)
        result = {"command": command, "exit_code": code,
                  "elapsed_seconds": time.perf_counter() - started}
        evidence["checks"][name] = result
        save()
        return result, stdout

    _, head = run("git-head", ["git", "rev-parse", "HEAD"])
    _, diff = run("git-diff", ["git", "diff", "--", "src", "Cargo.toml", "Cargo.lock"])
    _, rust = run("rust-version", ["rustc", "-Vv"])
    _, cargo = run("cargo-version", ["cargo", "-V"])
    evidence["environment"] = {"platform": platform.platform(), "machine": platform.machine(),
                               "cpu_count": os.cpu_count(), "python": platform.python_version(),
                               "rustc": rust.strip(), "cargo": cargo.strip(),
                               "git_head": head.strip(), "runtime_diff_sha256": hashlib.sha256(diff.encode()).hexdigest(),
                               "started_utc": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())}
    evidence["runtime_source_sha256"] = source_snapshot(checkout)
    source_groups, inline_ranges, unresolved_cfg = source_line_counts(checkout)
    evidence["rust_source_lines"] = source_groups
    evidence["source_line_definition"] = "Physical/nonblank Rust lines including comments; files under src/tests/** and complete exact #[cfg(test)] items/modules are tests. String/comment-aware item boundary scanner; compound test cfg attributes are flagged for review, not inferred. Mixed files count in both groups."
    evidence["inline_test_line_ranges"] = inline_ranges
    evidence["test_cfg_requiring_manual_classification"] = unresolved_cfg
    evidence["source_hash_coverage"] = {
        "scope": "All files recursively under src/ (including Rust, Lua/JSON fixtures) and .cargo/, plus existing Cargo.toml, Cargo.lock, build.rs, rustfmt.toml and .rustfmt.toml; symlink files hashed by their contents",
        "files_at_start": len(evidence["runtime_source_sha256"]),
        "enumeration_complete_for_scope": True,
        "limitations": "Not a hermetic build-input hash: does not follow symlink directories; excludes external path dependencies, system/user Cargo config, generated OUT_DIR contents, installed toolchain and assets outside the declared scope"}
    _, metadata = run("metadata", ["cargo", "metadata", "--locked", "--format-version", "1", "--no-deps"])
    package = next(p for p in json.loads(metadata)["packages"] if p["name"] == "cartridge")
    evidence["direct_dependencies"] = [d["name"] for d in package["dependencies"] if d["kind"] is None]
    _, tree = run("dependency-tree", ["cargo", "tree", "--locked", "--package", "cartridge", "--edges", "normal,build", "--prefix", "none", "--format", "{p}"])
    evidence["resolved_normal_and_build_dependency_count"] = len(set(
        line.replace(" (*)", "") for line in tree.splitlines() if line)) - 1
    evidence["lockfile_package_count_including_root_all_targets_and_dev"] = (checkout / "Cargo.lock").read_text().count("[[package]]")
    build, _ = run("release-build", ["cargo", "build", "--locked", "--release", "--package", "cartridge", "--bin", "cartridge", "--example", "rpc_fixture"])
    if build["exit_code"] != 0:
        raise RuntimeError("release build failed; see evidence logs")
    binary = target / "release/cartridge"
    sdk = target / "release/examples/rpc_fixture"
    evidence["release_binary"] = {"bytes": binary.stat().st_size,
                                  "sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
                                  "profile": "Cargo default --release, no extra RUSTFLAGS or stripping"}
    evidence["environment"]["inherited_rustflags"] = env.get("RUSTFLAGS")
    evidence["environment"]["inherited_encoded_rustflags"] = env.get("CARGO_ENCODED_RUSTFLAGS")
    _, filetype = run("release-file", ["file", str(binary)])
    evidence["release_binary"]["file"] = filetype.strip()

    payload = {"operation": "quality-roundtrip", "request": 7, "enabled": True,
               "values": list(range(16)), "text": "x" * 1024}
    evidence["workload_specification"] = {
        "transport": "persistent Unix socket, newline JSON, verified correlated replies",
        "payload": payload,
        "compact_json_payload_bytes": len(json.dumps(payload, separators=(",", ":")).encode()),
        "lua": "Python client -> release daemon -> Lua echo -> daemon -> Python client",
        "roundtrip": "Python client -> daemon -> Rust SDK child -> host Lua echo + two host metadata calls -> SDK child -> daemon -> Python client",
        "concurrency": [1, 16], "samples_per_workload": args.samples,
        "warmups_per_workload": args.warmups,
        "startup_samples": args.startup_samples, "startup_warmups": 0,
        "startup": "Popen of daemon to first validated SDK-to-Lua RPC; retries unavailable service every 1 ms, excludes fixture creation and socket-path lookup; repeated launches after build, caches uncontrolled, first launch included",
        "timing": "perf_counter_ns; latency includes socket send, daemon work, response read and Python JSON decode; request JSON encoding excluded; batch throughput includes Python request encoding and reply validation",
        "limitations": ["Same host micro-workload; no network, external tools or full agent session",
                        "Only native host architecture tested; scheduler and shared host load affect results",
                        "Python client contributes to observed latency and caps peak throughput",
                        "No sustained stress, queue growth, leak, large profile, hot reload, or disk-heavy workload coverage",
                        "Startup includes first launch and predominantly warm repeat launches, with 1 ms readiness polling granularity; cache state is not controlled",
                        "RSS snapshots cover daemon only, excluding SDK child and shared-library attribution"]}

    def start_daemon(folder):
        profile = Path(folder)
        (profile / "lua.lua").write_text('return {provide={"lua"},apply=function(ctx) ctx:provide("lua",function(args) return args end) end}\n')
        (profile / "peer.lua").write_text("return cartridge.process(" + json.dumps(str(sdk)) + ")\n")
        (profile / "init.lua").write_text('return {{id="lua",path="lua.lua"},{id="peer",path="peer.lua"}}\n')
        command = [str(binary), "--dir", folder, "--profile", folder]
        socket_path = subprocess.check_output(command + ["socket"], env=env, text=True).strip()
        log = open(profile / "daemon.stderr.log", "w")
        begun = time.perf_counter_ns()
        process = subprocess.Popen(command + ["daemon"], env=env, cwd=folder,
                                   stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
                                   stderr=log, start_new_session=True)
        connection = None
        try:
            deadline = time.monotonic() + 20
            while time.monotonic() < deadline:
                if process.poll() is not None:
                    raise RuntimeError("daemon exited: " + (profile / "daemon.stderr.log").read_text())
                try:
                    if connection is None:
                        connection = Connection(socket_path)
                    connection.batch("roundtrip", payload, 1)
                    return process, connection, log, socket_path, (time.perf_counter_ns() - begun) / 1e6
                except (OSError, RuntimeError):
                    time.sleep(.001)
            raise RuntimeError("readiness deadline elapsed: " + (profile / "daemon.stderr.log").read_text())
        except BaseException:
            stop_daemon(process, connection, log, socket_path)
            raise

    def stop_daemon(process, connection, log, socket_path):
        if connection:
            connection.close()
        try:
            os.killpg(process.pid, signal.SIGTERM)
        except ProcessLookupError:
            pass
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            os.killpg(process.pid, signal.SIGKILL)
            process.wait(timeout=5)
        log.close()
        Path(socket_path).unlink(missing_ok=True)

    startup = []
    print("Measuring startup", flush=True)
    for _ in range(args.startup_samples):
        with tempfile.TemporaryDirectory(prefix="cq-", dir="/tmp") as folder:
            process, connection, log, socket_path, elapsed = start_daemon(folder)
            startup.append(elapsed)
            stop_daemon(process, connection, log, socket_path)
    evidence["startup"] = dict(summary(startup), raw_ms=startup)
    save()
    with tempfile.TemporaryDirectory(prefix="cq-", dir="/tmp") as folder:
        process, connection, log, socket_path, _ = start_daemon(folder)
        try:
            for concurrency in (1, 16):
                for key in ("lua", "roundtrip"):
                    name = key + "_concurrency_" + str(concurrency)
                    print("Measuring " + name, flush=True)
                    for _ in range(math.ceil(args.warmups / concurrency)):
                        connection.batch(key, payload, concurrency)
                    samples = []
                    begun = time.perf_counter()
                    while len(samples) < args.samples:
                        samples.extend(connection.batch(key, payload, min(concurrency, args.samples - len(samples))))
                    elapsed = time.perf_counter() - begun
                    evidence["workloads"][name] = dict(summary(samples), raw_ms=samples,
                        actual_warmups=math.ceil(args.warmups / concurrency) * concurrency,
                        elapsed_seconds=elapsed, completed_calls_per_second=len(samples) / elapsed)
                    save()
            rss = subprocess.check_output(["ps", "-o", "rss=", "-p", str(process.pid)], text=True).strip()
            evidence["daemon_rss_after_workload_kib"] = int(rss)
        finally:
            stop_daemon(process, connection, log, socket_path)
    if args.checks:
        run("fmt", ["cargo", "fmt", "--all", "--", "--check"])
        run("clippy", ["cargo", "clippy", "--locked", "--all-targets", "--", "-D", "warnings"])
        run("tests", ["cargo", "test", "--locked", "--all-targets"])
    current_hashes = source_snapshot(checkout)
    original_hashes = evidence["runtime_source_sha256"]
    evidence["source_hash_coverage"]["files_at_end"] = len(current_hashes)
    evidence["source_changes_during_measurement"] = {
        "added": sorted(current_hashes.keys() - original_hashes.keys()),
        "removed": sorted(original_hashes.keys() - current_hashes.keys()),
        "modified": sorted(name for name in current_hashes.keys() & original_hashes.keys()
                           if current_hashes[name] != original_hashes[name])}
    evidence["source_unchanged_during_measurement"] = current_hashes == evidence["runtime_source_sha256"]
    evidence["completed_utc"] = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
    save()
    print(json.dumps({"output": str(output), "startup": summary(startup), "workloads": {
        key: {k: v for k, v in value.items() if k != "raw_ms"}
        for key, value in evidence["workloads"].items()}, "checks": evidence["checks"]}, indent=2))


if __name__ == "__main__":
    main()
