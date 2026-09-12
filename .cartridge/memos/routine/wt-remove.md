---
kind: routine
description: Delete a worktrunk worktree and, by default, its branch when the branch is already merged.
  Use to clean up a finished or abandoned branch; flags force out dirty trees, delete unmerged branches,
  or keep the branch.
uses:
- usage: '[[run-usage]]'
  when:
  - remove a worktree
  - delete a worktree and branch
  - clean up a merged branch
  - force remove a dirty worktree
  - delete an unmerged branch
  - keep branch after removing worktree
  tags:
  - worktrunk
  - wt
  - remove
  - delete
  - cleanup
  - branch
  - vcs
---

## Inputs

Requires on PATH: `wt`. Recipe parameters are named in Do; supply paths and arguments for the current task.
Risk: side effects git-write, file-write; danger high.

## Do

`wt remove [branches…]` deletes a worktree (default: the current one). It then
deletes the branch **only if merging it would add nothing** to the default branch —
it walks six equivalence checks, from same-commit to patch-id match. Unmerged
branches survive unless you force it. **Destructive**: removing a dirty tree or
forcing an unmerged branch drops work irrecoverably — confirm first.

| Recipe | Flags | Does |
|--------|-------|------|
| `remove` | — | remove the worktree; auto-delete the branch if merged |
| `force` | `-f` | remove even a dirty worktree (staged/modified/untracked) |
| `purge` | `-D` | also delete the branch even when unmerged |
| `keepbranch` | `--no-delete-branch` | remove the worktree, keep the branch |

`-f` (`--force`) is about a *dirty working tree*; `-D` (`--force-delete`) is about
an *unmerged branch* — different safety gates, both irreversible. `--foreground`
blocks until done instead of detaching; `--no-hooks` skips `pre/post-remove`
[[wt-hook]]. Default branch-deletion behaviour is `[remove]` in [[wt-config]].

```just
# remove the worktree (current if none named); delete branch if merged
remove branches="":
  wt remove {{branches}}

# force-remove a dirty worktree — drops uncommitted work
force branches="":
  wt remove -f {{branches}}

# also delete the branch even if it is unmerged
purge branch:
  wt remove -D {{quote(branch)}}

# remove the worktree but keep the branch
keepbranch branches="":
  wt remove --no-delete-branch {{branches}}
```

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `remove` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
