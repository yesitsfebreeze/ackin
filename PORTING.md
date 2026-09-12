# Port record

The source checkout at `~/dev/sys` was left in place. Its 15 cartridges and the
`landscape` support library were copied from tracked working-tree files into
independent sibling repositories. Each repository starts with an import commit
recording its source revision, followed by the changes for this layout.

The parent now contains only its 17 submodule directories and Git metadata.
All build scripts, documentation, profiles, catalogs, output, and validation
logs live in `cartridge.ctg/`. The shared 13-package cartridge workspace lives in
`cartridge.ctg/workspace/`; members explicitly point there with Cargo’s
`package.workspace` setting. The runtime and memory retain independent workspaces.

The runtime keeps its existing repository and history. Its name, CLI, Rust SDK,
Lua global, environment variables, and local state directory use `cartridge`.
Cartridge repositories use sibling Cargo dependencies. The runtime discovers
them through relative links under `cartridge.ctg/builtin/`.

Port-specific changes include:

- Restore the SDK reload registration used by agent and router.
- Restore the read-only composition snapshot used by memo, including nested
  forwarding and tests that exclude configuration values from the snapshot.
- Record offered and required services in each cartridge manifest.
- Move the shipped profiles and memo records to `.cartridge`.
- Adapt UI package names, shared wire fixtures, and palette expectations.
- Resolve the runtime composition directory separately from the Cargo workspace
  directory in the tooling cartridge; resolve linked manifests before Cargo runs.

## Validation

The composed checkout passed 1,723 Rust tests (148 runtime, 237 ported cartridge,
and 1,338 memory tests; 17 existing memory tests ignored) and 34 UI tests.
Runtime and cartridge Clippy passed with warnings denied. Memory's `just check`
passed formatting and strict Clippy across its workspace. The tooling contract
passed in both the main checkout and a newly provisioned runtime worktree; all
9 Python lane tests passed. All 15 cartridge links and declared dependency
bindings were checked through the launcher and ledger.

The memory adapter is enabled by default. Its standalone CLI can build with
`--no-default-features`; CI and release jobs fetch the sibling SDK, with the
Windows CLI build using that standalone mode. These workflow changes were
reviewed locally; remote CI and release publication have not run. The memory
Docker recipes mount the sibling SDK read-only and pass recipe expansion checks;
container execution was not tested because the local Docker daemon is stopped.

A subsequent packaging correction passed 6 focused tooling tests and strict
Clippy. Debug bundles built successfully from both the parent and runtime
working directories. The result contains all 15 manifests, the runtime, and 13
compiled cartridge executables matching their build outputs. Relocated outside
the workspace, that bundle resolved its ledger and executed `sessions.list`
through a real cartridge process. The debug bundle is at `dist/cartridge`; a full
release bundle has not been built or measured.

Before the final root cleanup, a fresh local recursive clone initialized all 17
submodules using local URL overrides. Its relative links and all three Cargo
workspace roots passed validation. The original runtime and all 14 linked
worktrees remain intact; existing uncommitted runtime files were preserved.

Raw local logs are kept under `validation/` and excluded from Git. Runtime
quality scores and subsequent fixes remain in
`.pearde/quality-review.md`.

## Git hosting

The submodule URLs name `yesitsfebreeze/<name>.ctg` on GitHub. Repositories and
commits are prepared locally; publishing them is a separate step. A local clone
can use Git's `submodule.<name>.url` configuration to point at these existing
checkouts. After publication, a recursive clone uses the recorded GitHub URLs.
