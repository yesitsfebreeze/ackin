---
kind: routine
description: Re-run a command on a fixed interval with watch — refresh its output every N seconds, highlight
  what changed between frames, or stop when the output changes. Use to monitor a value live, poll a status,
  or eyeball a changing file or directory.
uses:
- usage: '[[run-usage]]'
  when:
  - run a command repeatedly
  - refresh output every few seconds
  - monitor a value live
  - poll until something changes
  - highlight changes between runs
  - watch a directory listing
  tags:
  - watch
  - monitor
  - interval
  - poll
  - refresh
  - diff
---

## Inputs

Requires on PATH: `watch`. Recipe parameters are named in Do; supply paths and arguments for the current task.
Risk: danger none.

## Do

Re-runs a command on a clock and repaints the screen, so a one-shot command
becomes a live dashboard. `run` is the default fixed-interval repaint; `diff`
highlights the cells that changed since the last frame; `until` flips the logic
to *stop* the moment the output changes. For *event*-driven re-runs (on file
change, not on a timer) use [[watch-entr]] instead — watch is the timer; entr is
the trigger.

| Recipe | Flag | Behavior |
|--------|------|----------|
| `run` | `-n N` | re-run every N seconds, repaint |
| `diff` | `-d` | highlight differences from the previous frame |
| `until` | `-g` | exit as soon as the output changes (gate on change) |

`-n` accepts fractional seconds (`-n 0.5`). `-d` is what turns watch into a
change-spotter — the diff highlight catches a counter ticking or a status
flipping that you would miss in a wall of text. `-g` (`--chgexit`) is the
scripting trick: `watch -g 'ls *.lock'` blocks until the lock appears or
vanishes, then returns. `-t` drops the header for a cleaner capture.

```just
# re-run a command every N seconds and repaint (default)
run command interval="2":
  watch -n {{quote(interval)}} {{quote(command)}}

# highlight what changed since the previous frame
diff command interval="2":
  watch -d -n {{quote(interval)}} {{quote(command)}}

# exit as soon as the command's output changes
until command interval="2":
  watch -g -n {{quote(interval)}} {{quote(command)}}
```

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `run` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
