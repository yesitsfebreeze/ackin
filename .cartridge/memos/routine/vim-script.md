---
kind: routine
description: Run a vimscript or a Lua script through nvim headlessly. Use to execute editor automation
  that lives in a .vim or .lua file.
uses:
- usage: '[[run-usage]]'
  when:
  - run a vimscript
  - run nvim lua
  - execute editor automation
  - nvim -l
  - source a vim script headless
  tags:
  - vim
  - script
  - lua
  - vimscript
  - headless
---

## Inputs

Requires on PATH: `nvim`. Recipe parameters are named in Do; supply paths and arguments for the current task.
Risk: danger low.

## Do

`run` sources a vimscript with no UI and quits; `lua` runs a file through nvim's
own Lua interpreter (`nvim -l`), forwarding any args to the script. For one-off
ex commands against a file instead of a whole script, use [[vim-edit]]. Vimscript
options and mappings the script may set are documented in [[vim-config]].

```just
# source a vimscript headlessly, then quit (default)
run script:
  nvim --headless -S {{quote(script)}} -c 'qa!'

# run a Lua script through nvim's interpreter, forwarding args
lua script *args:
  nvim -l {{quote(script)}} {{args}}
```

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `run` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
