#!/usr/bin/env python3
"""Build and run the sibling cartridge repositories from any working directory."""
import json
import os
from pathlib import Path
import subprocess
import sys

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


def binary_links():
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
            raise RuntimeError(f"Expected built cartridge binary: {executable}")
        link = directory / "bin" / name
        link.parent.mkdir(exist_ok=True)
        if link.is_symlink():
            link.unlink()
        elif link.exists():
            raise RuntimeError(f"Refusing to replace a local executable: {link}")
        link.symlink_to(os.path.relpath(executable, link.parent))


def main():
    action = sys.argv[1] if len(sys.argv) > 1 else "help"
    if action == "help":
        print("workspace.py build | check | test | links | run <cartridge arguments>")
        return
    links()
    if action != "links":
        # Fixture builds spawn Cargo from the runtime's own directory. An
        # absolute environment value keeps those builds in this same cache.
        os.environ["CARGO_TARGET_DIR"] = str(target_directory())
    if action == "links":
        print("15 cartridge links and 16 sibling repositories verified")
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
        executable = target_directory() / "debug/cartridge"
        if not executable.is_file():
            raise RuntimeError("Build the workspace first: just build")
        os.chdir(RUNTIME)
        os.execv(str(executable), [str(executable), *sys.argv[2:]])
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
    except (RuntimeError, subprocess.CalledProcessError) as error:
        print(error, file=sys.stderr)
        sys.exit(1)
