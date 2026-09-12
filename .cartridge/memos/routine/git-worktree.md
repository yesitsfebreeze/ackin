---
kind: routine
description: Check out multiple branches at once in separate directories backed by one repo. Use to build,
  review, or hotfix a branch without disturbing the current tree.
uses:
- usage: '[[run-usage]]'
  when:
  - add a worktree
  - check out two branches at once
  - work on a branch without switching
  - list worktrees
  - remove a worktree
  tags:
  - git
  - worktree
  - checkout
  - vcs
---

## Inputs

Requires on PATH: `git`. Recipe parameters are named in Do; supply paths and arguments for the current task.
Risk: side effects git-write, file-write; danger low.

## Do

One repository, many working directories — each on its own branch, sharing the
same object store. Lets you hotfix or review a branch without a [[git-stash]] and
[[git-branch]] switch dance in the main tree. A branch checked out in one worktree
is locked from the others. `remove` deletes a worktree dir; `prune` clears stale
admin entries left by a deleted dir.

| Recipe | Does |
|--------|------|
| `add path branch` | new worktree at `path` on `branch` (default) |
| `list` | all worktrees and their branches |
| `remove path` | remove a worktree |
| `prune` | drop stale worktree metadata |

```just
# new worktree at path checked out to branch (default)
add path branch:
  git worktree add {{quote(path)}} {{quote(branch)}}

# all worktrees and their checked-out branches
list:
  git worktree list

# remove a worktree directory
remove path:
  git worktree remove {{quote(path)}}

# drop metadata for worktrees whose dirs are gone
prune:
  git worktree prune
```

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `add` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
