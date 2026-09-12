# Cartridge

The `cartridge` executable hosts replaceable cartridges. This repository owns
the runtime, Rust SDK, Lua composition API, resolver, event wire, build scripts,
and composition profiles.

```text
~/dev/cartridge/                 submodules and forwarding Justfile
  .gitmodules
  justfile                      forwards to cartridge.ctg/justfile
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

The parent contains Git metadata, its 17 submodules, and a forwarding Justfile. All 15 runnable
cartridges are linked under `builtin/`. The `landscape.ctg` sibling is a Rust
support library. [repositories.json](repositories.json) records all imported
repositories and their source revision; [PORTING.md](PORTING.md) records the port
and validation history. The original `~/dev/sys` checkout remains unchanged.

## Use

Requires Rust, Bun, Python 3.9+, Git, and optionally Just. From this repository:

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
build output inside the source repositories; the parent owns links and the forwarding Justfile.

The current checkout is initialized locally. GitHub URLs are configured, but
publication has not run. A local recursive clone can override submodule URLs to
these local repositories; after publication the recorded GitHub URLs can be used.

Run `pearde` here to continue the board. Independent critique scores and evidence
are in [.pearde/quality-review.md](.pearde/quality-review.md). Sandbox backend and
launch integration work remains open on the board.

## Development orchestration

The parent Justfile forwards to this repository, so these commands work from
`~/dev/cartridge` or `~/dev/cartridge/cartridge.ctg`:

```sh
just modules
just describe proxy
just test tools
just test runtime resolver
just test memory/store
just test ui tests/registry.test.ts
just check proxy
just build proxy
just smoke                 # isolated policy, proxy HTTP/auth, and MCP checks
just proxy                 # foreground proxy at http://127.0.0.1:4242
just proxy 4342             # choose another port
just mcp                   # MCP stdio; stdout contains only JSON-RPC
just process sessions hello
just invoke sessions '{"op":"list"}'
```

`just test` retains the full suite. A selected Rust module accepts normal Cargo
test arguments; UI accepts Bun test arguments. `memory/<package>` selects an
internal memory crate. Invalid names fail rather than falling back to all tests.
`just check <module>` runs formatting and strict Clippy for Rust modules; UI runs
TypeScript checking and the Lua policy runs its protocol checks.

Every repository has `development.json` with descriptions, services, dependencies,
and structured argument arrays. Command paths are relative to the repository
containing that JSON. SDK `process` commands speak JSON lines on stdin/stdout;
use `hello` to inspect their declared capabilities. `verify` applies only to
cartridges with a declared self-test contract and self-contained configuration.
The harness needs profile configuration; its development guide uses
`just invoke harness.selftest '{}' default` and `harness.integration` instead.

The proxy uses `CARTRIDGE_PROXY_KEY` when supplied, otherwise a private generated
key in `.cartridge/dev/proxy.key`. It derives a local profile from the shipped
proxy profile, selects the requested HTTP port and an available router port.
Provider credentials remain local. To connect a client, read that key into its
bearer-token setting; the launcher does not print the key. Ctrl-C stops the
foreground proxy. For another process to access its socket, use the generated
profile name `dev/proxy-4242`, for example `just status dev/proxy-4242`,
or pass that profile name as the final argument to `just call`.

MCP is client-owned: launch `just --justfile /absolute/path/to/cartridge.ctg/justfile mcp`
with stdin/stdout pipes and exchange newline-delimited JSON-RPC. No Codex client
configuration is changed automatically. The smoke command uses isolated state;
its proxy has a local router fixture and makes no model inference requests.

Development guidance and the native record were migrated from sys. See
[MIGRATION.md](MIGRATION.md) for preserved sessions, state and the compatibility link.
