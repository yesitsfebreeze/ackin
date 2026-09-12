---
kind: routine
description: Inspect network interfaces with ip — addresses, links, the routing table, and the default
  gateway. The modern ifconfig/route replacement. Use to find your IP, see which interface is up, read
  the routing table, or find the default gateway.
uses:
- usage: '[[run-usage]]'
  when:
  - what is my IP address
  - list network interfaces
  - show routing table
  - find the default gateway
  - is the interface up
  - replace ifconfig or route
  - which interface routes to the internet
  tags:
  - net
  - ip
  - interface
  - address
  - route
  - gateway
  - ifconfig
---

## Inputs

Requires on PATH: `ip`. Recipe parameters are named in Do; supply paths and arguments for the current task.
Risk: danger none.

## Do

The single tool that replaced `ifconfig`, `route`, and `arp`. `addr` (the
default) shows IP addresses per interface; `link` the layer-2 state (up/down,
MAC); `route` the full table; `gateway` extracts just the default route — the
"how do packets leave this box" answer. Pairs with [[net-ss]] (who is listening)
and [[net-dig]] (name resolution) to triage a connectivity problem end to end.

| Recipe | Subcommand | Shows |
|--------|-----------|-------|
| `addr` | `ip -br addr` | IP addresses per interface, one line each |
| `link` | `ip -br link` | interface up/down state and MAC |
| `route` | `ip route` | the full routing table |
| `gateway` | `ip route show default` | just the default gateway |

`-br` (brief) is the readable mode — a column per interface instead of the
verbose default. `ip addr` with no interface shows all; add a name
(`ip addr show eth0`) to narrow. The default route's `via` address is your
gateway; its `dev` is the interface traffic leaves on. `ip -s link` adds
RX/TX packet and error counters for spotting a flaky link.

```just
# IP addresses per interface, one brief line each (default)
addr:
  ip -br addr

# interface up/down state and MAC address
link:
  ip -br link

# the full routing table
route:
  ip route

# just the default gateway (how packets leave the box)
gateway:
  ip route show default
```

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `addr` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
