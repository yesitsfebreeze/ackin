---
kind: routine
description: Create, switch, list, delete, rename, and track branches. Use to manage local branches or
  move HEAD between them.
uses:
- usage: '[[run-usage]]'
  when:
  - create a branch
  - switch branches
  - list branches
  - delete a branch
  - rename a branch
  - set upstream
  tags:
  - git
  - branch
  - switch
  - vcs
---

## Inputs

Requires on PATH: `git`. Recipe parameters are named in Do; supply paths and arguments for the current task.
Risk: side effects git-write; danger low.

## Do

The branch surface around [[tools-commit]]. Names follow the ref grammar in
[[git-refs]]; `start` may be any commit-ish there. `create` makes a branch and
moves onto it; `switch` only moves HEAD (use [[git-stash]] first if the tree is
dirty). `delete` is safe (refuses unmerged) — force is a separate, deliberate
act. `track` wires a local branch to its remote so push/pull need no argument.

| Recipe | Does |
|--------|------|
| `create name start` | new branch at `start`, switch to it |
| `switch name` | move HEAD to an existing branch |
| `list` | local branches, current marked (default) |
| `delete name` | delete a merged branch |
| `rename old new` | rename a branch |
| `track name upstream` | set the upstream of `name` |

```just
# list local branches, current marked (default)
list:
  git branch --list -vv

# new branch at start-point, then switch to it
create name start="HEAD":
  git switch --create {{quote(name)}} {{quote(start)}}

# move HEAD to an existing branch
switch name:
  git switch {{quote(name)}}

# delete a merged branch (refuses if unmerged)
delete name:
  git branch --delete {{quote(name)}}

# rename a branch
rename old new:
  git branch --move {{quote(old)}} {{quote(new)}}

# set the upstream tracking ref of a branch
track name upstream:
  git branch --set-upstream-to {{quote(upstream)}} {{quote(name)}}
```

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `list` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
