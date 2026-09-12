---
kind: routine
description: Manage docker networks — list, create, inspect, remove, and connect a container to one. Use
  to wire containers together on a private network or to see and clean up existing networks.
uses:
- usage: '[[run-usage]]'
  when:
  - create a docker network
  - list networks
  - connect a container to a network
  - remove a network
  - inspect a network
  - wire containers together
  tags:
  - docker
  - network
  - connectivity
---

## Inputs

Requires on PATH: `docker`. Recipe parameters are named in Do; supply paths and arguments for the current task.
Risk: side effects network; danger medium.

## Do

Virtual networks that let containers reach each other by name. `list` is the
default; the rest are callable by name. A container attaches at run time with
`--network` ([[docker-cli]]) or after the fact via `connect`; the container itself
lives in [[docker-container]].

`rm` deletes a network and only succeeds once no container is attached. To sweep
every unused network, see [[docker-prune]].

| Recipe | Maps to | Notes |
|--------|---------|-------|
| `list` | `docker network ls` | all networks |
| `create` | `docker network create` | make a user-defined network |
| `rm` | `docker network rm` | delete a network |
| `inspect` | `docker network inspect` | subnet + attached containers as JSON |
| `connect` | `docker network connect` | attach a running container to a network |

```just
# list all networks (default)
list:
  docker network ls

# create a user-defined network
create name:
  docker network create -- {{quote(name)}}

# remove a network (must have no attached containers)
rm name:
  docker network rm -- {{quote(name)}}

# show a network's subnet and attached containers as JSON
inspect name:
  docker network inspect -- {{quote(name)}}

# attach a running container to a network
connect net container:
  docker network connect {{quote(net)}} {{quote(container)}}
```

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `list` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
