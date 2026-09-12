#!/usr/bin/env python3
"""Build and run the sibling cartridge repositories from any working directory."""
import json
import os
from pathlib import Path
import subprocess
import sys
import secrets
import socket

ROOT = Path(__file__).resolve().parent.parent
RUNTIME = ROOT
REPOSITORIES = (ROOT / "builtin/tools").resolve().parent
WORKSPACE = REPOSITORIES / "cartridge.ctg/workspace/Cargo.toml"


def run(args, cwd=ROOT, **kwargs):
    return subprocess.run(args, cwd=cwd, check=True, **kwargs)


def target_directory():
    result = run(["cargo", "metadata", "--no-deps", "--format-version", "1"],
                 capture_output=True, text=True)
    return Path(json.loads(result.stdout)["target_directory"])


def links():
    catalog = json.loads((ROOT / "repositories.json").read_text())
    for repository in catalog["repositories"]:
        directory = REPOSITORIES / repository["repository"]
        if not directory.is_dir():
            raise RuntimeError(f"Missing {directory.name}; initialize the Git submodules first")
        if repository["cartridge"]:
            link = RUNTIME / "builtin" / repository["name"]
            if not link.is_symlink() or link.resolve() != directory.resolve():
                raise RuntimeError(f"Invalid cartridge link: {link}")
            if not (link / "cartridge.json").is_file():
                raise RuntimeError(f"Missing cartridge manifest: {link}")


def memory_args(command):
    # Resolve before invoking Cargo: relative dependencies in a manifest reached
    # through builtin's symlink otherwise resolve against the link's directory.
    return ["cargo", command, "--manifest-path", str((REPOSITORIES / "memory.ctg/Cargo.toml").resolve()),
            "--target-dir", str(target_directory())]


def runtime_args(command):
    return ["cargo", command, "--manifest-path", str(RUNTIME / "Cargo.toml"),
            "--target-dir", str(target_directory())]


def workspace_args(command):
    return ["cargo", command, "--manifest-path", str(WORKSPACE),
            "--target-dir", str(target_directory())]


def binary_links(existing_only=False):
    target = target_directory() / "debug"
    catalog = json.loads((ROOT / "repositories.json").read_text())
    for repository in catalog["repositories"]:
        if not repository["cartridge"] or not repository["package"]:
            continue
        directory = REPOSITORIES / repository["repository"]
        manifest = json.loads((directory / "cartridge.json").read_text())
        name = manifest.get("binary", repository["name"])
        executable = target / name
        if not executable.is_file():
            if existing_only:
                continue
            raise RuntimeError(f"Expected built cartridge binary: {executable}")
        link = directory / "bin" / name
        link.parent.mkdir(exist_ok=True)
        if link.is_symlink():
            link.unlink()
        elif link.exists():
            raise RuntimeError(f"Refusing to replace a local executable: {link}")
        link.symlink_to(os.path.relpath(executable, link.parent))


def catalog():
    return json.loads((ROOT / "repositories.json").read_text())["repositories"]


def memory_modules():
    result = run(["cargo", "metadata", "--no-deps", "--format-version", "1",
                  "--manifest-path", str(REPOSITORIES / "memory.ctg/Cargo.toml")],
                 capture_output=True, text=True)
    return sorted(package["name"] for package in json.loads(result.stdout)["packages"])


def selected(action, target, extra):
    """Route names to the owning test/build system, preserving argument boundaries."""
    name = target.removesuffix(".ctg")
    if name in ("runtime", "cartridge"):
        args = runtime_args(action)
        scope = ["--all-targets"] if action != "build" else ["--bin", "cartridge"]
    elif name == "memory" or name.startswith("memory/"):
        args = memory_args(action)
        scope = ["--workspace"] if name == "memory" else ["-p", name.split("/", 1)[1]]
    else:
        repository = next((r for r in catalog() if name in (r["name"], r["package"])), None)
        if repository is None:
            raise RuntimeError(f"Unknown module {target!r}; use just modules")
        if name == "ui":
            if action == "build":
                return run(["bun", "install", "--frozen-lockfile", *extra], cwd=REPOSITORIES / "ui.ctg")
            return run(["bun", "run", action, *extra], cwd=REPOSITORIES / "ui.ctg")
        if repository["package"] is None:
            if extra:
                raise RuntimeError("Lua policy checks do not accept Cargo arguments")
            return run([sys.executable, str(ROOT / "scripts/smoke.py"), "policy"])
        args = workspace_args(action)
        scope = ["-p", repository["package"]]
    if action == "check":
        manifest = args[args.index("--manifest-path") + 1]
        formatting_scope = scope if scope[:1] == ["-p"] else ["--all"]
        run(["cargo", "fmt", "--manifest-path", manifest, *formatting_scope, "--", "--check"])
        args[1] = "clippy"
        if "--all-targets" not in scope:
            scope.append("--all-targets")
        return run(args + scope + [*extra, "--", "-D", "warnings"])
    return run(args + scope + extra)


def execute(arguments):
    executable = target_directory() / "debug/cartridge"
    if not executable.is_file():
        raise RuntimeError("Build the workspace first: just build")
    os.chdir(RUNTIME)
    os.execv(str(executable), [str(executable), *arguments])


def main():
    action = sys.argv[1] if len(sys.argv) > 1 else "help"
    if action == "help":
        print("workspace.py build|check|test [module] [args...] | modules | describe <module> | proxy | mcp | links | run <args...>")
        return
    if action in ("modules", "describe"):
        entries = [("runtime", ROOT), *((r["name"], REPOSITORIES / r["repository"]) for r in catalog())]
        for name, directory in entries:
            manifest = directory / "cartridge.json"
            if action == "describe":
                if len(sys.argv) != 3:
                    raise RuntimeError("Usage: just describe <module>")
                if sys.argv[2].removesuffix(".ctg") not in (name, "cartridge" if name == "runtime" else name):
                    continue
                document = manifest if manifest.is_file() else directory / "README.md"
                print(document.read_text(), end="")
                return
            else:
                description = (json.loads(manifest.read_text()).get("description", name)
                               if manifest.is_file() else f"See {directory.name}/README.md")
                print(f"{name:14} {description}")
        if action == "describe":
            raise RuntimeError(f"Unknown module {sys.argv[2]!r}; use just modules")
        print("\nMemory workspace modules (just test memory/<name>):")
        print("  " + ", ".join(memory_modules()))
        return
    links()
    if action != "links":
        # Fixture builds spawn Cargo from the runtime's own directory. An
        # absolute environment value keeps those builds in this same cache.
        os.environ["CARGO_TARGET_DIR"] = str(target_directory())
    if action == "links":
        print("15 cartridge links and 16 sibling repositories verified")
    elif action in ("build", "check", "test") and len(sys.argv) > 2 and sys.argv[2] != "all":
        selected(action, sys.argv[2], sys.argv[3:])
        if action == "build":
            binary_links(existing_only=True)
    elif action in ("build", "check", "test") and len(sys.argv) > 3:
        raise RuntimeError("Select a module before passing runner-specific arguments")
    elif action == "build":
        run(runtime_args("build") + ["--bin", "cartridge"])
        run(workspace_args("build") + ["--workspace", "--bins"])
        run(memory_args("build") + ["--bin", "memory", "--bin", "memory_cartridge"])
        run(["bun", "install", "--frozen-lockfile"], cwd=REPOSITORIES / "ui.ctg")
        binary_links()
    elif action == "check":
        run(["cargo", "fmt", "--manifest-path", str(WORKSPACE), "--all", "--", "--check"])
        run(["cargo", "fmt", "--manifest-path", str(RUNTIME / "Cargo.toml"), "--", "--check"])
        run(runtime_args("clippy") + ["--all-targets", "--", "-D", "warnings"])
        run(workspace_args("clippy") + ["--workspace", "--all-targets", "--", "-D", "warnings"])
        run(memory_args("check") + ["--workspace", "--all-targets"])
        run(["bun", "run", "check"], cwd=REPOSITORIES / "ui.ctg")
    elif action == "test":
        run(runtime_args("test") + ["--all-targets"])
        run(workspace_args("test") + ["--workspace"])
        run(memory_args("test") + ["--workspace"])
        run(["bun", "run", "test"], cwd=REPOSITORIES / "ui.ctg")
    elif action == "run":
        execute(sys.argv[2:])
    elif action == "mcp":
        execute(["--profile", "mcp", "mcp"])
    elif action == "process":
        if len(sys.argv) < 3:
            raise RuntimeError("Usage: just process <module> [hello]")
        name = sys.argv[2].removesuffix(".ctg")
        repository = next((r for r in catalog() if r["name"] == name and r["package"] and r["cartridge"]), None)
        if repository is None:
            raise RuntimeError(f"{name!r} is not a Rust process cartridge; use just describe {name}")
        manifest = json.loads((REPOSITORIES / repository["repository"] / "cartridge.json").read_text())
        binary = target_directory() / "debug" / manifest.get("binary", name)
        if not binary.is_file():
            raise RuntimeError(f"Build this module first: just build {name}")
        os.chdir(ROOT)
        os.execv(str(binary), [str(binary), *sys.argv[3:]])
    elif action == "proxy":
        port = int(sys.argv[2]) if len(sys.argv) > 2 else 4242
        if not 1 <= port <= 65535:
            raise RuntimeError("Proxy port must be between 1 and 65535")
        try:
            with socket.socket() as probe:
                probe.bind(("127.0.0.1", port))
        except OSError as error:
            raise RuntimeError(f"Proxy port {port} is unavailable; select another with just proxy <port>") from error
        if not os.environ.get("CARTRIDGE_PROXY_KEY"):
            directory = ROOT / ".cartridge/dev"
            directory.mkdir(mode=0o700, parents=True, exist_ok=True)
            key = directory / "proxy.key"
            try:
                fd = os.open(key, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
            except FileExistsError:
                pass
            else:
                with os.fdopen(fd, "w") as stream:
                    stream.write(secrets.token_hex(32) + "\n")
            os.environ["CARTRIDGE_PROXY_KEY"] = key.read_text().strip()
            if not os.environ["CARTRIDGE_PROXY_KEY"]:
                raise RuntimeError(f"Empty proxy key file: {key}")
            print(f"Proxy key file: {key}", file=sys.stderr)
        profile = ROOT / ".cartridge/dev" / f"proxy-{port}"
        profile.mkdir(parents=True, exist_ok=True)
        base = ROOT / ".cartridge/proxy"
        (profile / "init.lua").write_text((base / "init.lua").read_text())
        (profile / "config.lua").write_text(
            "local config = dofile(" + json.dumps(str(base / "config.lua")) + ")\n"
            + f'config.proxy.listen = "127.0.0.1:{port}"\n'
            + 'config.router.listen = {"127.0.0.1:0"}\nreturn config\n')
        print(f"Proxy: http://127.0.0.1:{port} (Ctrl-C to stop)", file=sys.stderr)
        execute(["--profile", str(profile), "daemon"])
    else:
        raise RuntimeError(f"Unknown workspace action: {action}")


if __name__ == "__main__":
    # Independent memory builds use the same compact development profile.
    os.environ.setdefault("CARGO_PROFILE_DEV_DEBUG", "0")
    os.environ.setdefault("CARGO_PROFILE_DEV_INCREMENTAL", "false")
    os.environ.setdefault("CARGO_PROFILE_TEST_DEBUG", "0")
    os.environ.setdefault("CARGO_PROFILE_TEST_INCREMENTAL", "false")
    try:
        main()
    except (RuntimeError, ValueError, subprocess.CalledProcessError) as error:
        print(error, file=sys.stderr)
        sys.exit(1)
