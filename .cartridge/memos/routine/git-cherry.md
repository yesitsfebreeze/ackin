---
kind: routine
description: Apply or undo individual commits — cherry-pick a commit onto HEAD, or revert one with an
  inverse commit. Use to port a fix between branches or back out a change.
uses:
- usage: '[[run-usage]]'
  when:
  - cherry-pick a commit
  - port a fix to another branch
  - revert a commit
  - undo a commit safely
  - apply a single commit
  tags:
  - git
  - cherry-pick
  - revert
  - vcs
---

## Inputs

Requires on PATH: `git`. Recipe parameters are named in Do; supply paths and arguments for the current task.
Risk: side effects git-write; danger medium.

## Do

Move single commits, not whole branches. `pick` replays a commit (any
commit-ish, [[git-refs]]) onto the current branch as a new commit — the way to port
a fix without a [[git-rebase]]. `revert` records the inverse of a commit as a new
commit, so it undoes a change without rewriting history (safe on shared
branches, unlike reset — see [[git-recover]]). Either can conflict: resolve, then
`continue`; `abort` unwinds. Find the target SHA with [[git-history]].

| Recipe | Does |
|--------|------|
| `pick rev` | replay `rev` onto HEAD as a new commit (default) |
| `revert rev` | commit the inverse of `rev` |
| `continue` | resume after resolving a conflict |
| `abort` | unwind the in-progress pick/revert |

```just
# replay a commit onto HEAD as a new commit (default)
pick rev:
  git cherry-pick {{quote(rev)}}

# record the inverse of a commit as a new commit
revert rev:
  git revert --no-edit {{quote(rev)}}

# resume after resolving a conflict
continue:
  git cherry-pick --continue

# unwind the in-progress pick or revert
abort:
  git cherry-pick --abort
```

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `pick` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
