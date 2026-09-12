---
kind: routine
description: Take lines from the start or end of a file with head and tail — the first N, the last N,
  everything after a line, or a live follow of an appending file. Use to peek at a file's start or end,
  skip a header, or watch a log grow in real time.
uses:
- usage: '[[run-usage]]'
  when:
  - show the first lines of a file
  - show the last lines
  - watch a log file live
  - tail -f
  - skip the header row
  - peek at the end of a file
  - follow an appending file
  tags:
  - text
  - tail
  - head
  - lines
  - follow
  - log
---

## Inputs

Requires on PATH: `coreutils`. Recipe parameters are named in Do; supply paths and arguments for the current task.
Risk: danger none.

## Do

Slice a file by line position. `last`/`first` take N lines from the end/start;
`after` drops a leading header; `follow` streams new lines as they are appended
— the live-log view. `follow` runs until interrupted (a streaming command, not a
one-shot); the rest return immediately. Pair with [[search-rg]] to filter the lines
and [[text-sort]] once you have the slice.

| Recipe | Does |
|--------|------|
| `last` | the last N lines (default 10) |
| `first` | the first N lines |
| `after` | everything from line N onward (skip a header with `+2`) |
| `follow` | stream appended lines live (`tail -f`) until interrupted |

`tail -n +2` (the `after` form) skips line 1 — the idiom for dropping a CSV
header before a pipeline. `follow` adds `-F` (capital) so it survives log
rotation by reopening the path; plain `-f` follows the file descriptor and goes
silent when the file is rotated out.

```just
# the last N lines (default)
last n="10" file="-":
  tail -n {{quote(n)}} -- {{quote(file)}}

# the first N lines
first n="10" file="-":
  head -n {{quote(n)}} -- {{quote(file)}}

# everything from line N onward (e.g. n="+2" skips a header)
after n file="-":
  tail -n {{quote(n)}} -- {{quote(file)}}

# stream appended lines live, surviving rotation (runs until interrupted)
follow file:
  tail -F -- {{quote(file)}}
```

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `last` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
