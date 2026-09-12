---
kind: routine
description: Observe a running container — follow logs, inspect its full config as JSON, watch live resource
  stats, and see its in-container processes. Use to debug why a container misbehaves without changing
  anything.
uses:
- usage: '[[run-usage]]'
  when:
  - follow container logs
  - tail docker logs
  - inspect a container
  - container config json
  - docker stats
  - resource usage
  - processes in a container
  - debug a container
  tags:
  - docker
  - logs
  - inspect
  - stats
  - debug
---

## Inputs

Requires on PATH: `docker`. Recipe parameters are named in Do; supply paths and arguments for the current task.
Risk: danger none.

## Do

Read-only views into a running container — nothing here mutates state. `logs` is
the default; the rest are callable by name. Targets are container names or ids
([[docker-cli]]); the container itself is managed in [[docker-container]].

| Recipe | Maps to | Notes |
|--------|---------|-------|
| `logs` | `docker logs -f --tail` | follow output, last `tail` lines first |
| `inspect` | `docker inspect` | full config + state as JSON |
| `stats` | `docker stats --no-stream` | one CPU/memory/IO snapshot |
| `top` | `docker top` | processes running inside the container |

`inspect` returns JSON — pipe it to `jq` to pull a single field. `stats` uses
`--no-stream` for a single snapshot; drop it for a live dashboard.

```just
# follow a container's logs, showing the last `tail` lines first (default)
logs name tail="100":
  docker logs -f --tail {{quote(tail)}} -- {{quote(name)}}

# dump a container's full config and state as JSON
inspect name:
  docker inspect -- {{quote(name)}}

# one snapshot of a container's CPU, memory, and IO
stats name:
  docker stats --no-stream -- {{quote(name)}}

# list the processes running inside a container
top name:
  docker top -- {{quote(name)}}
```

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `logs` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
