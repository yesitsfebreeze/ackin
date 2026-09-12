---
kind: note
description: "What a fresh checkout needs to run every gate, and where the memory workspace's source came from"
date: "2026-09-12"
---

# Fresh-checkout gates

A clean checkout runs `just check` and `just test` with no setup step. Every
cartridge's source is in the tree, the memory workspace included: it is a
separate cargo workspace at `builtin/memory/`, built by manifest
(`just memory-build`) rather than as a workspace member, and tracked like any
other source.

Provenance of that workspace, since it did not start here: it is the Kern
engine from https://github.com/yesitsfebreeze/kern at revision
`58850707f37f7cd10939b742c4c9898e84552193`, plus the cartridge adapter that
runs it inside the host SDK process (`cartridge.json`, `init.lua`,
`src/cartridge.rs`, `tests/cartridge.rs`) and the edits its store and command
modules needed. That revision is no longer reachable from the remote — the
history was rewritten — so this repository is where the pinned tree survives.
Until 2026-09-12 it survived twice, as a vendored copy plus a patch that
reconstructed the adapter on demand; [[the-memory-workspace-is-tracked-not-patched]]
records why that collapsed into ordinary tracked files.

The workspace's `target/` and its own record clone stay ignored, so neither
enters this repository.

Toolchain prerequisites, recorded from the verified fresh-checkout run:
rustc 1.94.0 / cargo 1.94.0, cargo-nextest 0.9.143, Bun 1.3.14,
Python 3.14.6, plus nu/zsh/bash for the shell and PTY fixtures. Lockfiles are
respected: `Cargo.lock`, `builtin/memory/Cargo.lock` and `builtin/ui/bun.lock`
are tracked, and the recipes use `--frozen-lockfile`.
