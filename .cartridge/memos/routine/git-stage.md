---
kind: routine
description: Manage the staging area — stage changes, unstage them, discard working-tree edits, and diff
  staged vs unstaged. The index surface that sits between your edits and a commit. Use to choose what
  goes into the next commit, undo a stage, or throw away a change.
uses:
- usage: '[[run-usage]]'
  when:
  - stage changes for commit
  - unstage a file
  - discard working tree changes
  - what is staged
  - diff staged changes
  - undo git add
  - see unstaged changes
  tags:
  - git
  - stage
  - index
  - add
  - reset
  - diff
  - vcs
---

## Inputs

Requires on PATH: `git`. Recipe parameters are named in Do; supply paths and arguments for the current task.
Risk: side effects git-write; danger medium.

## Do

The index — the layer between your working tree and the next [[tools-commit]].
`add` moves changes in; `unstage` moves them back out (keeping the edits);
`discard` throws working-tree edits away (destructive); `diff` shows staged vs
unstaged. Names follow [[git-refs]]. The split between *unstage* (safe, keeps your
work) and *discard* (irreversible) is the one to keep straight.

| Recipe | Does |
|--------|------|
| `status` | short view of staged / unstaged / untracked |
| `add` | stage a path (or `.` for everything) |
| `unstage` | remove from the index, **keep** the working-tree edit |
| `discard` | throw away an unstaged working-tree edit (irreversible) |
| `diff` | staged vs unstaged changes (`--staged` for the index) |

`unstage` is `git restore --staged` — safe, your edits remain. `discard` is
`git restore` (no `--staged`) — it overwrites the file from the index and the
change is gone, so confirm before running it. `add -p` (patch mode) stages
hunks selectively; the recipe here stages whole paths. `diff` with no args shows
*unstaged*; `diff --staged` shows what a commit would capture.

```just
# short status: staged / unstaged / untracked (default)
status:
  git status --short

# stage a path (use "." for everything)
add path=".":
  git add -- {{quote(path)}}

# unstage a path but KEEP the working-tree edit (safe)
unstage path:
  git restore --staged -- {{quote(path)}}

# discard an unstaged working-tree edit (IRREVERSIBLE)
discard path:
  git restore -- {{quote(path)}}

# diff; pass which="--staged" to see what a commit would capture
diff which="":
  git diff {{which}}
```

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `status` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
