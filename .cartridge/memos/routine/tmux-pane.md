---
kind: routine
description: Manage tmux panes within a window — split, select, resize, zoom, kill, layout. Use to tile
  a window into multiple shells and arrange them.
uses:
- usage: '[[run-usage]]'
  when:
  - split a tmux pane
  - resize tmux pane
  - zoom a tmux pane
  - kill a pane
  - change tmux layout
  tags:
  - tmux
  - pane
  - split
  - layout
---

## Inputs

Requires on PATH: `tmux`. Recipe parameters are named in Do; supply paths and arguments for the current task.
Risk: side effects process; danger low.

## Do

A pane is one shell; panes tile a window ([[tmux-window]]). `split` is the default
and splits left/right (`-h`); pass `vert=v` for top/bottom (`-v`). The `target`
is a [[tmux-cli]] address down to `session:window.pane`; once split, run programs in
a pane with [[tmux-send]].

```just
# split the target pane; dir h=left/right (default), v=top/bottom
split target="" dir="h":
  tmux split-window -{{dir}} -t {{quote(target)}}

# focus a pane (target may be a direction like {left}/{down})
select target:
  tmux select-pane -t {{quote(target)}}

# resize a pane; dir U/D/L/R, amount in cells
resize target dir amount="5":
  tmux resize-pane -t {{quote(target)}} -{{dir}} {{quote(amount)}}

# toggle zoom — fullscreen the pane within its window
zoom target:
  tmux resize-pane -Z -t {{quote(target)}}

# kill a pane
kill target:
  tmux kill-pane -t {{quote(target)}}

# apply a preset layout to the window
# (even-horizontal | even-vertical | main-horizontal | main-vertical | tiled)
layout target name="tiled":
  tmux select-layout -t {{quote(target)}} {{quote(name)}}
```

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `split` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
