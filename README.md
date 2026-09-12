# Cartridge

The `cartridge` executable hosts replaceable cartridges. This repository owns
the runtime, Rust SDK, Lua composition API, resolver, event wire, build scripts,
and composition profiles.

```text
~/dev/cartridge/                 Git submodule linker only
  .gitmodules
  cartridge.ctg/                 this source repository
    src/                         runtime and SDK
    builtin/agent -> ../../agent.ctg
    .cartridge/                  shipped profiles; local stores ignored
    workspace/Cargo.toml         shared workspace for the Rust cartridges
    scripts/workspace.py         build and validation commands
    target/                      ignored shared build output
    dist/                        ignored bundles
    validation/                  ignored local check logs
  agent.ctg/                     sibling cartridge repository
  …                              15 other imported sibling repositories
```

The parent contains only Git metadata and its 17 submodules. All 15 runnable
cartridges are linked under `builtin/`. The `landscape.ctg` sibling is a Rust
support library. [repositories.json](repositories.json) records all imported
repositories and their source revision; [PORTING.md](PORTING.md) records the port
and validation history. The original `~/dev/sys` checkout remains unchanged.

## Use

Requires Rust, Bun, Python 3, Git, and optionally Just. From this repository:

```sh
just build
./cartridge ledger
./cartridge run ui '{}'
```

Without Just, use `python3 scripts/workspace.py build`. The launcher works from
any working directory and runs the application here, giving profiles and stores
stable paths. Provider operations require your own configuration; credentials
and live stores were not copied from `~/dev/sys`.

`just check`, `just test`, and `just links` validate the composed checkout.
The runtime keeps its standalone Cargo workspace for independent builds and
Pearde worktrees. The shared cartridge workspace is in `workspace/`; its members
are the sibling repositories. Memory keeps its own workspace and lockfile.
The helper shares one target directory across all three workspaces and creates
ignored relative `bin/` links for process cartridges. Development builds omit
debug symbols and incremental caches to bound disk use.

To build a distributable bundle:

```sh
./cartridge --profile tools run tools '{"op":"bundle","profile":"release"}'
```

The bundle is written to `dist/cartridge/`. Use `"debug"` for the development
profile. Packaging checks that every required executable is present.

## Work on the repositories

Commit implementation changes inside the affected submodule, then commit the
new submodule pointer in the parent. Keep source, documentation, tooling, and
build output inside the source repositories; the parent is only the linker.

The current checkout is initialized locally. GitHub URLs are configured, but
publication has not run. A local recursive clone can override submodule URLs to
these local repositories; after publication the recorded GitHub URLs can be used.

Run `pearde` here to continue the board. Independent critique scores and evidence
are in [.pearde/quality-review.md](.pearde/quality-review.md). Sandbox backend and
launch integration work remains open on the board.
