---
kind: routine
description: Test reachability and trace the path to a host — ping a fixed count, trace the hops with
  traceroute, or watch a live per-hop quality report with mtr. Use to check if a host is up, measure latency,
  or find where on the path packets are lost.
uses:
- usage: '[[run-usage]]'
  when:
  - is this host reachable
  - measure latency to a host
  - trace the network path
  - where are packets being lost
  - ping a host
  - traceroute
  - diagnose slow connection
  tags:
  - net
  - ping
  - traceroute
  - mtr
  - latency
  - reachability
---

## Inputs

Requires on PATH: `ping`. Recipe parameters are named in Do; supply paths and arguments for the current task.
Risk: danger none.

## Do

Reachability and path diagnosis. `check` (the default) sends a fixed count and
reports loss + round-trip time; `trace` lists the hops to the target; `mtr`
combines ping and traceroute into a live per-hop loss/latency table — the best
single view of *where* a path degrades. Use after [[net-ip]] confirms you have a
route and [[net-dig]] confirms the name resolves: this tests the path actually
works.

| Recipe | Tool | Shows |
|--------|------|-------|
| `check` | `ping -c N` | loss % and min/avg/max RTT over N packets |
| `trace` | `traceroute` | each hop (router) on the way to the host |
| `mtr` | `mtr --report` | per-hop loss and latency in one combined report |
| `flood` | `ping -c N -i 0.2` | rapid probes to surface intermittent loss |

`check` always bounds the count (`-c`) so it terminates — a bare `ping` runs
forever. In the RTT summary, a high *max* with a low *avg* means jitter; steady
high *avg* means distance or congestion. `mtr` is the diagnostic of choice: the
first hop with sustained loss that persists to the destination is the culprit
(transient loss at a *middle* hop that clears downstream is just a router
deprioritizing ICMP — not your problem).

```just
# loss % and round-trip time over N packets (default)
[unix, wsl]
check host count="5":
  ping -c {{quote(count)}} -- {{quote(host)}}
[macos]
check host count="5":
  ping -c {{quote(count)}} {{quote(host)}}
[windows]
check host count="5":
  ping -n {{quote(count)}} {{quote(host)}}

# list the hops on the path to a host
[unix, macos, wsl]
trace host:
  traceroute {{quote(host)}}
[windows]
trace host:
  tracert {{quote(host)}}

# live per-hop loss and latency report (mtr on unix/macOS, pathping on Windows)
[unix, macos, wsl]
mtr host:
  mtr --report --report-cycles 10 {{quote(host)}}
[windows]
mtr host:
  pathping {{quote(host)}}

# rapid probes to surface intermittent loss
[unix, wsl]
flood host count="20":
  ping -c {{quote(count)}} -i 0.2 -- {{quote(host)}}
[macos]
flood host count="20":
  ping -c {{quote(count)}} -i 0.2 {{quote(host)}}
[windows]
flood host count="20":
  ping -n {{quote(count)}} {{quote(host)}}
```

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `check` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
