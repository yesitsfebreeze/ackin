---
kind: routine
description: Shelve and restore uncommitted changes — push, pop, list, show, drop, apply a stash. Use
  to park work-in-progress before switching branches or pulling.
uses:
- usage: '[[run-usage]]'
  when:
  - stash changes
  - shelve work in progress
  - pop a stash
  - list stashes
  - restore stashed changes
  tags:
  - git
  - stash
  - wip
  - vcs
---

## Inputs

Requires on PATH: `git`. Recipe parameters are named in Do; supply paths and arguments for the current task.
Risk: side effects git-write; danger low.

## Do

Parks a dirty tree onto a LIFO stack so [[git-branch]] switch or a pull runs
clean. Entries are addressed as `stash@{n}` (see the `@{...}` syntax in
[[git-refs]]). `pop` applies the top entry and removes it; `apply` keeps it.
`drop` discards one — a dropped stash is still recoverable for a while
([[git-recover]]).

| Recipe | Does |
|--------|------|
| `push msg` | shelve tracked changes with a label (default) |
| `pop ref` | apply newest (or `ref`) and remove it |
| `apply ref` | apply but keep the entry |
| `list` | the stash stack |
| `show ref` | diffstat of an entry |
| `drop ref` | discard one entry |

```just
# shelve current changes with a label (default)
push msg="wip":
  git stash push --message {{quote(msg)}}

# apply the top entry (or a given ref) and remove it
pop ref="stash@{0}":
  git stash pop {{quote(ref)}}

# apply an entry but keep it on the stack
apply ref="stash@{0}":
  git stash apply {{quote(ref)}}

# the stash stack
list:
  git stash list

# diffstat of one entry
show ref="stash@{0}":
  git stash show --stat {{quote(ref)}}

# discard one entry
drop ref="stash@{0}":
  git stash drop {{quote(ref)}}
```

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `push` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
