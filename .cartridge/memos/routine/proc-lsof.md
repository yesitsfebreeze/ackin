---
kind: routine
description: List open files and sockets with lsof — what holds a file or mountpoint, which process owns
  a TCP/UDP port, and every file a PID has open. Use to free a busy mount, find what listens on a port,
  or trace a process's resources.
uses:
- usage: '[[run-usage]]'
  when:
  - what is using this port
  - what process holds this file
  - free a busy mountpoint
  - list open files of a process
  - who is listening on a port
  - find process by port
  tags:
  - proc
  - lsof
  - ports
  - sockets
  - open-files
  - network
---

## Inputs

Requires on PATH: `lsof`. Recipe parameters are named in Do; supply paths and arguments for the current task.
Risk: danger none.

## Do

"List open files" — and on Unix nearly everything is a file, so this answers
*what is using a port*, *what holds this mount busy*, and *what has this PID
opened*. The PIDs it returns pair with [[proc-ps]] to identify and [[shell-jobs]] to
signal. The classic use is freeing a port before a restart or a mount before an
unmount.

| Recipe | Query | Answers |
|--------|-------|---------|
| `port` | `-i :N` | which process owns a TCP/UDP port |
| `file` | path | which processes hold a file or directory open |
| `pid` | `-p` | every file a given process has open |
| `listen` | `-iTCP -sTCP:LISTEN` | all listening TCP sockets |

`port 8080` is the daily driver — it names the PID squatting on a port so you
can free it. `file /mnt/x` finds what keeps a mount busy when `umount` says
"target is busy". Add `-t` to any of these to get bare PIDs for piping into
`kill` (`lsof -t -i:8080 | xargs -r kill`). May need `sudo` to see other users'
files.

```just
# what process owns a TCP/UDP port (default)
port number:
  lsof -i :{{quote(number)}}

# which processes hold a file or directory open (frees a busy mount)
file path:
  lsof -- {{quote(path)}}

# every open file of a given process
pid id:
  lsof -p {{quote(id)}}

# all listening TCP sockets
listen:
  lsof -iTCP -sTCP:LISTEN -P -n
```

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `port` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
