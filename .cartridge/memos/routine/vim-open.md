---
kind: routine
description: Launch vim with the right invocation — at a line, in tabs or splits, read-only, restoring
  a session, or with no config. Use to open files in vim the way the task needs.
uses:
- usage: '[[run-usage]]'
  when:
  - open a file in vim
  - open at a line
  - vim tabs
  - vim splits
  - restore a vim session
  - vim clean config
  tags:
  - vim
  - open
  - launch
  - tabs
  - splits
---

## Inputs

Requires on PATH: `nvim`. Recipe parameters are named in Do; supply paths and arguments for the current task.
Risk: danger none.

## Do

Thin wrappers over vim's launch flags. The full flag and binary-family reference
is [[vim-cli]]; window layout once open is [[vim-windows]]. `open` is the default; the
rest pick a layout or mode.

```just
# open one or more files (default)
open +files:
  nvim -- {{files}}

# open a file with the cursor on a line
at file line:
  nvim {{quote("+" + line)}} -- {{quote(file)}}

# open each file in its own tab
tabs +files:
  nvim -p -- {{files}}

# open files in vertical splits
split +files:
  nvim -O -- {{files}}

# open a file read-only (view)
readonly file:
  nvim -R -- {{quote(file)}}

# restore a saved session
session file="Session.vim":
  nvim -S {{quote(file)}}

# launch with no config or plugins — stock defaults only
clean:
  nvim --clean
```

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `open` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
