---
kind: routine
description: Manage tmux windows within a session — create, select, rename, kill, list, move. Use to organize
  a session into tabbed windows.
uses:
- usage: '[[run-usage]]'
  when:
  - new tmux window
  - switch tmux window
  - rename tmux window
  - kill tmux window
  - move a tmux window
  tags:
  - tmux
  - window
  - tab
  - session
---

## Inputs

Requires on PATH: `tmux`. Recipe parameters are named in Do; supply paths and arguments for the current task.
Risk: side effects process; danger low.

## Do

A window is a tabbed full-screen view inside a session, holding one or more
panes ([[tmux-pane]]). `new` is the default. The `target` arg is a [[tmux-cli]]
address (`session` or `session:window`); operate on the live program in a window
with [[tmux-send]].

```just
# create a new window in a session (default)
new target="main" name="":
  tmux new-window -t {{quote(target)}} -n {{quote(name)}}

# select (focus) a window by target
select target:
  tmux select-window -t {{quote(target)}}

# rename a window
rename target newname:
  tmux rename-window -t {{quote(target)}} {{quote(newname)}}

# kill a window
kill target:
  tmux kill-window -t {{quote(target)}}

# list windows, by default of the current session
list target="":
  tmux list-windows -t {{quote(target)}}

# move/renumber a window to a destination target (-k overwrites if occupied)
move src dst:
  tmux move-window -k -s {{quote(src)}} -t {{quote(dst)}}
```

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `new` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
