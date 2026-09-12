---
kind: routine
description: The burrito terminal multiplexer — spawn a grid of PTY shells, descend recursively into any
  pane, and run headless behind a unix socket for thin-client attach.
uses:
- usage: '[[run-usage]]'
  when:
  - launch burrito
  - start burrito
  - reattach burrito session
  - run burrito with options
  - burrito launch panes
  - burrito --help
  - what can burrito do
  tags:
  - burrito
  - terminal
  - multiplexer
  - pty
  - grid
  - session
---

## Inputs

Requires on PATH: `burrito`. Recipe parameters are named in Do; supply paths and arguments for the current task.
Risk: side effects process, file-write; danger low.

## Do

burrito is a terminal multiplexer that nests inside itself. Boots a cols×rows
grid of PTY shells (2×2 up to 4×4, default 3×3), renders them all at once,
and recurses: descend into any pane to start another burrito with its own grid,
to any depth.

Two modes:
- **Headed** (default): `burrito` reattaches the nearest live session for this
  cwd, or spawns one. The grid runs in the host terminal with raw mode + alt
  screen; the active pane's I/O is byte-exact (Transparency).
- **Headless** (opt-in): `burrito --new`, `--name`, or `--attach` starts a
  daemon server behind a unix socket. Thin clients attach with `burrito attach
  <socket>` and stream frames (full cells or sparse diffs).

Session resolution: cwd walks up to a git root (nearest `.git` ancestor) for the
session key, or falls back to the bare cwd. `--name` overrides with a global
named session. `--new` forces a fresh session regardless of any live one.

| Recipe | Does |
|--------|------|
| `default` | reattach nearest live session for this cwd, or spawn one |
| `new` | force a fresh isolated session (skip reattach) |
| `named n="work"` | attach/create a global named session |
| `attach socket` | attach a thin client to a running session's socket |
| `help` | print the embedded tutorial + exit |

The comma-separated `slots` parameter to `launch` is the pane number (1‑based)
before the `>`, so `launch slots="3:htop,5:nvim ."` fills tiles 3 and 5.

```just
# reattach or spawn for this cwd (default)
default:
  burrito

# force a fresh isolated session
new:
  burrito --new

# attach/create a global named session
named name="work":
  burrito --name {{quote(name)}}

# attach a thin client to a running session's socket
attach socket:
  burrito attach {{quote(socket)}}

# print the embedded tutorial
help:
  burrito --help

# list live sessions (name, cwd, socket)
list:
  burrito ls

# launch with a 4×4 grid on a named session
named-4x4 name="work":
  burrito -w 4 -h 4 --name {{quote(name)}}

# prefill panes at boot: "slot:cmd,slot:cmd"
launch slots="":
  burrito {{ if slots != "" { " " + split(slots, ",") | map(join(" --launch ", "")) | join(" ") } else { "" } }}
```

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `default` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
