---
kind: work
description: "The gates run in a lane, where the independently versioned memory workspace is absent"
status: done
uses:
  - usage: "[[read-usage]]"
    when: ["just check or just test fails in a worktree with 'manifest path builtin/memory/Cargo.toml does not exist'"]
---

# gates-run-in-a-lane

## Outcome

`just check` and `just test` pass in a lane worktree, so work can be proven
before it lands.

`builtin/memory` is an independently versioned workspace with its own git
repository, ignored by this one. A lane created by `just lane` therefore has no
`builtin/memory/Cargo.toml`, and both gates depend on `memory-check` and
`memory-test`, which fail immediately with `manifest path
builtin/memory/Cargo.toml does not exist`. Every gate in every lane dies before
reaching this repository's own checks.

The memory workspace has its own checks and its own history; a lane is not the
place they run. What must not happen is a gate that appears to pass while
silently skipping them.

## Approach

Skipping the memory checks in a lane was tried first and rejected: a lane also
runs `core/tests/profile.rs`, which builds the memory cartridge, so skipping the
gate only moved the failure. The lane needs the workspace, not an exemption.

1. `just lane` lends the new worktree the trunk's `builtin/memory` as a symlink.
2. The two terminal tests ask cargo for its target directory instead of assuming
   `ROOT/target`, which `.cargo/config.toml` contradicts in every worktree.
3. The justfile's own `target` variable had the same wrong default, so the tools
   tests ran with `ZIRKLE_WRAPPER` pointing at a binary no lane has. It asks cargo
   too.

## Check

- [x] `just all` passes inside a lane created by `just lane`: 211/211 Rust
      tests, 11 bun tests, 6 tools tests, 2 core terminal tests. First ticked
      after checking the gate's parts by hand, which missed that `just test`
      still died at the tools tests; `56dd640` fixed that and the box was
      re-earned by running `just all` end to end.
- [x] Both still run the memory workspace's checks in the trunk, where its
      manifest exists. The lane borrows the trunk's copy, so the checks run
      there too: clippy reported `Checking kern v2.0.0`.
- [x] Nothing is silently skipped. The workspace is lent to the lane rather
      than stepped over, so the memory checks always run.

## Result

Landed as `141afed` and `56dd640`. `just lane` lends the trunk's memory workspace to each new
worktree, and `core/tests/test_development.py` and `core/tests/test_ui.py`
resolve the binary through `cargo metadata`. Verified in the lane before
landing: `just check` green, 211/211 Rust tests, doc tests, `test_ui.py`, 11 bun
tests.

Two things this exposed, recorded separately: [[lane-rm-refuses-after-the-gates-run]].
