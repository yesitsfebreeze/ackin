---
kind: routine
description: Read the history — log, commit graph, show a commit, blame lines, and the reflog. Use to
  inspect what changed, when, by whom, and to find lost commits.
uses:
- usage: '[[run-usage]]'
  when:
  - show the log
  - view commit history
  - commit graph
  - blame a file
  - who changed this line
  - show a commit
  - view the reflog
  tags:
  - git
  - log
  - history
  - blame
  - reflog
  - vcs
---

## Inputs

Requires on PATH: `git`. Recipe parameters are named in Do; supply paths and arguments for the current task.
Risk: danger none.

## Do

Read-only inspection. Ranges and revisions are ref grammar ([[git-refs]]): pass
`main..HEAD` to `log`, a `path:line` to `blame`. `reflog` records every move of
HEAD — the recovery anchor for a lost commit or a bad [[git-rebase]] ([[git-recover]]
shows how to use it). `graph` renders the topology of merges and branches.

| Recipe | Does |
|--------|------|
| `log rev` | one-line log of `rev` (default `HEAD`) |
| `graph rev` | ASCII commit graph across all branches |
| `show rev` | full diff of one commit |
| `blame file` | annotate each line with its last commit |
| `reflog` | every recent position of HEAD |

```just
# one-line log of a revision or range (default HEAD)
log rev="HEAD":
  git log --oneline --decorate -n 30 {{quote(rev)}}

# ASCII commit graph across all branches
graph rev="--all":
  git log --oneline --graph --decorate {{quote(rev)}}

# full diff of a single commit
show rev="HEAD":
  git show {{quote(rev)}}

# annotate each line of a file with its last-touching commit
blame file:
  git blame {{quote(file)}}

# every recent position of HEAD (recovery anchor)
reflog:
  git reflog
```

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `log` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
