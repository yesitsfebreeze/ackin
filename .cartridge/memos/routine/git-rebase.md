---
kind: routine
description: Replay commits onto a new base, edit history interactively, or autosquash fixups. Use to
  linearize, reorder, squash, or reword commits before sharing.
uses:
- usage: '[[run-usage]]'
  when:
  - rebase onto main
  - interactive rebase
  - squash commits
  - reorder commits
  - reword history
  - continue a rebase
  - abort a rebase
  tags:
  - git
  - rebase
  - history
  - vcs
---

## Inputs

Requires on PATH: `git`. Recipe parameters are named in Do; supply paths and arguments for the current task.
Risk: side effects git-write, history-rewrite; danger medium.

## Do

Rewrites history: every replayed commit gets a new SHA, so never rebase a
branch others have pulled. Base and range are ref grammar ([[git-refs]]). A
conflict pauses the replay — resolve, then `continue`; `abort` restores the
pre-rebase state (which also lives in `ORIG_HEAD`, see [[git-recover]]).
`autosquash` consumes `fixup!`/`squash!` commits from [[tools-commit]]
automatically. Pair with [[git-history]] to inspect before and after.

| Recipe | Does |
|--------|------|
| `onto base` | replay current branch on top of `base` (default) |
| `interactive base` | open the todo editor (pick/reword/squash/drop) |
| `autosquash base` | fold fixup!/squash! commits, non-interactive |
| `continue` | resume after resolving a conflict |
| `abort` | unwind, restore the original branch |

```just
# replay the current branch onto a new base (default)
onto base:
  git rebase {{quote(base)}}

# open the interactive todo editor over commits since base
interactive base:
  git rebase --interactive {{quote(base)}}

# fold fixup!/squash! commits into their targets
autosquash base:
  git rebase --interactive --autosquash {{quote(base)}}

# resume after resolving a conflict
continue:
  git rebase --continue

# unwind and restore the original branch
abort:
  git rebase --abort
```

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `onto` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
