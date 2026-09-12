---
kind: routine
description: Inspect running processes with ps — snapshot all processes, find by name, sort by CPU or
  memory, and show a process tree. Use to see what is running, hunt a runaway process, or find a PID to
  signal.
uses:
- usage: '[[run-usage]]'
  when:
  - list running processes
  - find a process by name
  - what is using CPU
  - what is using memory
  - get a PID
  - show process tree
  - top memory consumers
  tags:
  - proc
  - ps
  - process
  - cpu
  - memory
  - pid
---

## Inputs

Requires on PATH: `ps`. Recipe parameters are named in Do; supply paths and arguments for the current task.
Risk: danger none.

## Do

A point-in-time snapshot of processes (the moving-picture view is `top`/`htop`).
`all` is the default wide listing; the rest find by name or rank by resource.
The PIDs it yields feed the signals in [[shell-jobs]] (`kill PID`) and the open-file
lookups in [[proc-lsof]]. For a live updating view prefer `htop`; ps is for
scripting and one-shot questions.

| Recipe | Sort/filter | Shows |
|--------|-------------|-------|
| `all` | — | every process, wide format |
| `grep` | name match | processes whose command matches |
| `cpu` | `--sort=-%cpu` | top CPU consumers |
| `mem` | `--sort=-%mem` | top memory consumers |
| `tree` | `--forest` | the parent/child process tree |

`grep` filters in-process (no second `grep` that matches itself). `cpu`/`mem`
take an optional count of rows to show. Columns: `%cpu`/`%mem` are percentages,
`rss` is resident memory in KB, `etime` is elapsed run time.

```just
# every process, wide BSD-style listing (default)
all:
  ps aux

# processes whose command matches a pattern (no self-match)
grep pattern:
  #!/usr/bin/env sh
  set -eu
  ps aux | grep -- {{quote(pattern)}} | grep -v grep

# top N CPU consumers
cpu n="10":
  ps aux --sort=-%cpu | head -n {{quote(n)}}

# top N memory consumers
mem n="10":
  ps aux --sort=-%mem | head -n {{quote(n)}}

# the process tree (parent/child forest)
tree:
  ps -e --forest -o pid,ppid,user,cmd
```

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `all` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
