---
kind: routine
description: Manage burrito sessions — resolve, attach, detach, kill, list live sessions, and inspect
  session metadata. Sessions are cwd-keyed (git root → path) or explicitly named.
uses:
- usage: '[[run-usage]]'
  when:
  - list burrito sessions
  - kill a burrito session
  - detach burrito
  - attach to a burrito session
  - find live burrito sessions
  - burrito session management
  - burrito ls
  tags:
  - burrito
  - session
  - attach
  - detach
  - kill
  - list
  - daemon
  - named
---

## Inputs

Requires on PATH: `burrito`. Recipe parameters are named in Do; supply paths and arguments for the current task.
Risk: side effects process, file-write, file-delete; danger low.

## Do

burrito sessions are either cwd-keyed (walks up to the nearest git root, or
uses the bare cwd) or explicitly named with `--name`. Sessions run as daemon
servers behind a unix socket in `$XDG_RUNTIME_DIR/burrito/` (or
`$TMPDIR/burrito-$uid/`). Each session has a companion `.rc.sock` control
socket and a `.meta` sidecar with the display name + cwd.

## Display names

Path-keyed sessions get a deterministic `adjective-noun` name (e.g.
`calm-otter`, `brave-falcon`) derived from the hash of the session key, so the
same path always resolves to the same name across detach/reattach without any
persistence.

```just
# list every live session (name, cwd, socket path — tab-separated)
list:
  burrito ls

# attach a thin client to a running session socket (copy from `burrito ls`)
attach socket:
  burrito attach {{quote(socket)}}

# kill a session by killing its server (from inside: `burrito ctl detach`)
# inside the burrito exit menu: [k]ill kills the session
kill:
  @echo "open the exit menu (leader esc) and press k, or use pkill burrito"
```

For remote control of a running session use [[burrito-ctl]]. For the window-tiling
subcommand use [[burrito-wm]].

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `list` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
