---
kind: routine
description: Find who last changed each line with git blame — annotate a file, blame a specific line range,
  ignore whitespace-only churn, and trace a line through a move. Use to find the commit and author behind
  a line, or when a change was introduced.
uses:
- usage: '[[run-usage]]'
  when:
  - who changed this line
  - when was this line added
  - find the commit for a line
  - blame a file
  - who wrote this code
  - trace a line through refactors
  tags:
  - git
  - blame
  - annotate
  - authorship
  - history
  - vcs
---

## Inputs

Requires on PATH: `git`. Recipe parameters are named in Do; supply paths and arguments for the current task.
Risk: danger low.

## Do

Per-line authorship: the commit, author, and date that last touched each line.
`file` annotates the whole file; `range` narrows to a span; `ignorews` skips
whitespace-only changes so a reformat does not mask the real author; `commit`
jumps from a blamed hash to its full diff. Names follow [[git-refs]]; the commit it
surfaces feeds [[git-history]] for the surrounding context.

| Recipe | Flags | Does |
|--------|-------|------|
| `file` | — | annotate every line with its last commit |
| `range` | `-L a,b` | blame only lines a–b |
| `ignorews` | `-w` | ignore whitespace-only edits |
| `commit` | `show` | open the full diff of a blamed hash |

`-w` is essential after a reformat — without it, blame credits the formatting
commit for every line. `-C` and `-M` follow lines through copies and moves, so a
function that was relocated still blames its original author. The hash blame
prints is short; feed it to `commit` (a `git show`) for the why. To blame as of
an older point, add a revision: `git blame <rev> -- file`.

```just
# annotate every line with its last commit/author/date (default)
file path:
  git blame -- {{quote(path)}}

# blame only a line range (e.g. start="40" end="60")
range start end path:
  git blame -L {{quote(start)}},{{quote(end)}} -- {{quote(path)}}

# ignore whitespace-only changes (see past a reformat)
ignorews path:
  git blame -w -M -C -- {{quote(path)}}

# open the full diff of a blamed commit hash
commit hash:
  git show {{quote(hash)}}
```

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `file` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
