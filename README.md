# Cartridge

The `cartridge` executable hosts replaceable cartridges. This repository contains
the runtime, Rust SDK, Lua composition API, dependency resolver, and event wire.

The parent `cartridge` checkout links this repository and the cartridge
repositories as sibling Git submodules. `builtin/` contains relative links to
those siblings; `.cartridge/` contains the composition profiles.

From the parent directory:

```sh
just build
./cartridge ledger
./cartridge run ui '{}'
```

This runtime has its own Cargo workspace so it can be built and tested alone,
including in Pearde worktrees. The parent helper shares its target directory
with the cartridge builds. Each process cartridge's ignored `bin/` link points
to its built executable, including when the runtime runs from a worktree.

Run `pearde` from this repository to continue its board. Review history is in
`.pearde/quality-review.md`. Sandboxing work still has open backend and launch
integration items; the board records their current verification status.
