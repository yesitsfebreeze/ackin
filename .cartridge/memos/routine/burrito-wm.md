---
kind: routine
description: Drive the cross-platform window engine (`burrito-wm`) with burrito's own grid model — `burrito
  wm list | tile <col> <row> [window_id]`. Reuses the 3×3 grid coordinates that place PTY panes to tile
  real OS windows.
uses:
- usage: '[[run-usage]]'
  when:
  - burrito wm
  - tile windows with burrito
  - list windows burrito
  - burrito window management
  - window tiling burrito
  tags:
  - burrito
  - wm
  - window-manager
  - tiling
  - x11
  - wayland
  - windows
---

## Inputs

Requires on PATH: `burrito`. Recipe parameters are named in Do; supply paths and arguments for the current task.
Risk: side effects process; danger low.

## Do

`burrito wm list | tile` drives the cross-platform window engine
(`burrito-wm`). The same `col,row` coordinates that place PTY panes tile real
OS windows in a 3×3 grid. The backend is chosen per-platform/compositor; on
Wayland (where foreign-window control is restricted) it says so rather than lie.

Coordinates are **0-indexed** and bounded to 0..2 (3×3 grid).

```just
# list all managed windows with their hex id and title
list:
  burrito wm list

# tile the first window into cell (1,1) — the centre of the 3×3
tile col="1" row="1":
  burrito wm tile {{col}} {{row}}

# tile a specific window by hex id into cell (2,0)
tile-id col="2" row="0" id="0x0a0b0c0d":
  burrito wm tile {{col}} {{row}} {{quote(id)}}

# default: list
default:
  burrito wm list
```

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `list` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
