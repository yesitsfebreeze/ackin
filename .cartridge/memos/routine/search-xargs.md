---
kind: routine
description: Build and run commands from a list on stdin with xargs — one invocation for many args, one-per-line,
  placeholder substitution, and parallel fan-out. The glue between a finder and an action. Use to batch
  a command over found paths, safely handle spaces, or parallelize.
uses:
- usage: '[[run-usage]]'
  when:
  - run command on each line of output
  - batch process found files
  - xargs parallel
  - handle filenames with spaces
  - pipe find into a command
  - one command per input
  - placeholder substitution
  tags:
  - search
  - xargs
  - batch
  - pipeline
  - parallel
  - stdin
---

## Inputs

Requires on PATH: `xargs`. Recipe parameters are named in Do; supply paths and arguments for the current task.
Risk: side effects file-write; danger medium.

## Do

Turns a stream of lines into command arguments — the standard glue between a
finder ([[search-find]], [[search-rg]] `-l`) and the action you want per result. The
safe idiom pairs `-0` here with a null-separated producer so spaces and newlines
in names never split an argument. `run` is the default (append args once);
`each` runs once per input; `parallel` fans out.

| Recipe | Flags | Behavior |
|--------|-------|----------|
| `run` | — | collect all lines, append as args to one command |
| `each` | `-I{}` | one invocation per line, `{}` is the line |
| `null` | `-0` | split on NUL, not whitespace (the safe form) |
| `parallel` | `-P -0` | up to N invocations at once, null-safe |

Whitespace in filenames is the classic footgun: a plain `find ... | xargs rm`
breaks on `my file.txt`. Always produce NUL-separated output (`find -print0`,
`rg -0`, `fd -0`) and consume it with `-0`. `-r` (`--no-run-if-empty`) skips
running on empty input.

```just
# collect all stdin lines and append them as args to a command (default)
run +command:
  xargs -r {{command}}

# run the command once per input line; {} is the line
each +command:
  xargs -r -I{} {{command}}

# NUL-separated input (pair with find -print0 / rg -0 / fd -0) — handles spaces
null +command:
  xargs -0 -r {{command}}

# run up to `jobs` commands in parallel, NUL-separated and safe
parallel jobs +command:
  xargs -0 -r -P {{quote(jobs)}} {{command}}
```

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `run` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
