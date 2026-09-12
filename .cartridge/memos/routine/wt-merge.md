---
kind: routine
description: Land a worktrunk branch in one shot — stage, squash into one commit, rebase onto the target,
  run verify hooks, fast-forward the target, then remove the worktree. Use to ship a finished branch;
  tune which phases run with the --no-* flags.
uses:
- usage: '[[run-usage]]'
  when:
  - merge a worktree branch
  - land a finished branch
  - squash and fast-forward to main
  - ship a branch and clean up
  - merge without removing the worktree
  tags:
  - worktrunk
  - wt
  - merge
  - land
  - squash
  - rebase
  - ship
  - vcs
---

## Inputs

Requires on PATH: `wt`. Recipe parameters are named in Do; supply paths and arguments for the current task.
Risk: side effects git-write, file-write; danger high.

## Do

`wt merge [target]` runs the whole landing pipeline: stage uncommitted work →
squash the branch into one commit → rebase onto `target` (default: the default
branch) → run `pre-merge` [[wt-hook]] checks → fast-forward the target → remove the
worktree ([[wt-remove]]). Target defaults to `^`. **Destructive**: it rewrites the
branch via squash/rebase and deletes the worktree, so confirm before running.

| Recipe | Flags | Does |
|--------|-------|------|
| `merge` | — | full pipeline: squash → rebase → verify → ff → remove |
| `keep` | `--no-remove` | land it but leave the worktree in place |
| `nosquash` | `--no-squash` | keep individual commits instead of one squash |

Each phase has a `--no-*` off switch: `--no-squash`, `--no-rebase` (fail if not
already rebased), `--no-ff` (make a merge commit), `--no-commit` (don't auto-commit
dirty changes), `--no-remove`, `--no-hooks`. `--stage all|tracked|none` controls
what gets picked up before the auto-commit. `-y` skips the approval prompt; defaults
live under `[merge]` in [[wt-config]].

```just
# full pipeline: squash → rebase → verify → fast-forward → remove worktree
merge target="":
  wt merge {{target}}

# land it but keep the worktree afterwards
keep target="":
  wt merge --no-remove {{target}}

# preserve individual commits (no squash)
nosquash target="":
  wt merge --no-squash {{target}}
```

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `merge` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
