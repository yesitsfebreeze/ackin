---
kind: routine
description: Inspect network sockets with ss — listening ports, established connections, owning process,
  and per-port lookups. The modern netstat replacement. Use to see what is listening, who is connected,
  or which process holds a port.
uses:
- usage: '[[run-usage]]'
  when:
  - what ports are listening
  - show network connections
  - which process owns a port
  - established connections
  - replace netstat
  - count connections by state
  tags:
  - net
  - ss
  - sockets
  - ports
  - netstat
  - connections
---

## Inputs

Requires on PATH: `ss`. Recipe parameters are named in Do; supply paths and arguments for the current task.
Risk: danger none.

## Do

Socket statistics — the fast successor to `netstat`. `listen` (the default)
shows what is accepting connections; the rest widen to all sockets, filter by
port, or summarize. It overlaps [[proc-lsof]] on "what owns a port" but is faster
and connection-state aware; reach for lsof when you also want the open *files*,
ss when you want the *sockets*. PIDs it surfaces feed [[proc-ps]] and [[shell-jobs]].

| Recipe | Flags | Shows |
|--------|-------|-------|
| `listen` | `-tlnp` | listening TCP ports + owning process |
| `all` | `-tunap` | all TCP+UDP sockets, numeric, with process |
| `port` | `( sport = :N )` | sockets on a specific port |
| `summary` | `-s` | totals by protocol and state |

The flag letters: `t` TCP, `u` UDP, `l` listening, `n` numeric (no DNS/port
name lookups — much faster), `p` process, `a` all states. Seeing the process
column (`-p`) usually needs `sudo`. Filter expressions like `state established`
or `dport = :443` narrow further.

```just
# listening TCP ports with the owning process (default)
listen:
  ss -tlnp

# all TCP and UDP sockets, numeric, with process
all:
  ss -tunap

# sockets bound to or talking to a specific port
port number:
  ss -tunap "( sport = :{{number}} or dport = :{{number}} )"

# summary totals by protocol and connection state
summary:
  ss -s
```

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `listen` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
