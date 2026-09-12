---
kind: work
description: "A clean checkout can bootstrap every declared gate dependency"
status: active
owner: "sys-work-2026-09-12/analyst-fresh-checkouts-can-run-the-gates"
level: 10
priority: P1
estimate: 1d
needs:
  - "[[terminal-profile-starts-only-needed-services]]"
---

# A clean checkout can bootstrap every declared gate dependency

## Outcome

A new checkout can reproduce the native and optional memory gates from documented, pinned inputs without borrowing another worktree or an undocumented local repository.

## Check

- [ ] In a temporary clean checkout, documented bootstrap and native gates succeed without a preexisting builtin/memory directory or trunk symlink.
- [ ] The optional recall workspace has an explicit source and revision plus a reproducible setup command; its gate reports a missing dependency as a failure when requested.
- [ ] Toolchain versions and prerequisites for Rust, nextest, Bun, Python and terminal fixtures are recorded; lockfiles are respected.

## Approach

Observed 2026-09-12: `.gitignore` excludes `builtin/memory`; `just check` and `just test` require it. `just lane` borrows the trunk copy. That solves local worktrees, not a fresh clone.

Audit: [[runtime-audit-2026-09-12]].

## Spec

The probe is committed on `work/fresh-checkouts-can-run-the-gates` (5 commits,
HEAD `b9fefe3`, branched before the momo→zirkle rename). The bootstrap is built
and verified; the implementer runs it and ticks the checks.

Facts found by building:

- `builtin/memory` is the Kern engine at
  `https://github.com/yesitsfebreeze/kern` revision
  `58850707f37f7cd10939b742c4c9898e84552193` plus a Momo cartridge adapter that
  is not upstream: it is the diff `58850707..c7226c94` carried on the memory
  repo's `work/momo-memory-cartridge` branch.
- That pinned revision is **not reachable from the remote** (`git fetch` of the
  SHA answers "not our ref"; history was rewritten), so a fresh clone cannot
  `git clone` it and an upstream-SHA bootstrap fails at the source. A
  reproducible bootstrap must carry the tree itself: the pinned tree is
  vendored (`6.0M`, 405 files) and the adapter is tracked as one patch.

Files the change touches:

- `justfile` — new `memory-bootstrap` recipe (copies the vendored tree into
  `builtin/memory`, applies the adapter patch, and commits a root import so
  the nested repository is real); a `memory-present` guard that fails with
  "run `just memory-bootstrap`" when the workspace is missing; the
  `memory-build`/`memory-check`/`memory-test` recipes depend on the guard.
- `builtin/memory-cartridge.patch` — the 14-file adapter, regenerable with
  `git diff 58850707..c7226c94`, edited to fix `clippy::nonminimal_bool`
  under rustc 1.94.
- `vendor/kern-58850707f37f7cd10939b742c4c9898e84552193/` — the pinned tree,
  regenerable with `git -C builtin/memory archive 58850707... | tar -x`.
- `.zirkle/memos/note/memory-bootstrap.md` (was `.momo/memos/`) — the recorded
  source, revision, setup command and toolchain prerequisites (checks two and
  three).

Steps in order (the hand runs these):

1. Rebase/refresh the lane onto the current trunk first (it branched before
   the momo→zirkle rename; the justfile has since moved `-p momo` → `-p zirkle` and
   the record moved to `.zirkle/memos/`).
2. Ensure `just memory-bootstrap` is in the lane's `justfile` and the vendored
   tree + patch are tracked, then run it in a temporary clean checkout.
3. `just check` — probe-verified green (exit 0) on a fresh clone of the lane,
   with memory and root workspaces recompiled from clone sources.
4. `just test` — probe-verified: memory workspace 1337 and root workspace 249
   tests run green except that one pre-existing timing-sensitive test (kern
   e2e retention, watcher debounce, or a pty subprocess timeout) trips per
   full run under shared-target parallel load; each passes in isolation in the
   same clone, and the audit logged the same memory suite passing on this pin.
5. `bun install --frozen-lockfile` needs network once; `memory-bootstrap`
   itself needs none.

Remaining defined work: a full-suite pass under no parallel contention (run
`just test` with the machine otherwise idle, or re-run after a flaked run);
nothing in the bootstrap itself is left undone.
