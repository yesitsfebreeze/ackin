---
kind: routine
description: Manage tmux sessions — create, attach, list, rename, kill. Use to start or tear down a named
  workspace the server keeps alive in the background.
uses:
- usage: '[[run-usage]]'
  when:
  - start a tmux session
  - create a detached session
  - attach to tmux
  - list tmux sessions
  - kill a tmux session
  tags:
  - tmux
  - session
  - workspace
  - server
---

## Inputs

Requires on PATH: `tmux`. Recipe parameters are named in Do; supply paths and arguments for the current task.
Risk: side effects process; danger low.

## Do

A session is a named workspace the server keeps running whether or not a client
is attached. `new` is the default; the rest are callable by name. `new` creates
it detached (`-d`) so a script can keep going, then drive it with [[tmux-send]] or
attach later. Targets follow [[tmux-cli]]; windows inside live in [[tmux-window]].

```just
# create a new detached session (default); name it for stable targeting
new name="main":
  tmux new-session -d -s {{quote(name)}}

# attach the current terminal to an existing session
attach name="main":
  tmux attach-session -t {{quote(name)}}

# list every session the server holds
list:
  tmux list-sessions

# rename a session
rename name newname:
  tmux rename-session -t {{quote(name)}} {{quote(newname)}}

# kill a session and everything in it
kill name:
  tmux kill-session -t {{quote(name)}}
```

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `new` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
