---
kind: routine
description: Manage named volumes — list, create, inspect, and remove the persistent storage containers
  mount. Use to set up storage that outlives a container, see what volumes exist, or reclaim one.
uses:
- usage: '[[run-usage]]'
  when:
  - create a docker volume
  - list volumes
  - persistent storage for a container
  - remove a volume
  - inspect a volume
  - named volume
  tags:
  - docker
  - volume
  - storage
  - persistence
---

## Inputs

Requires on PATH: `docker`. Recipe parameters are named in Do; supply paths and arguments for the current task.
Risk: side effects disk, data-loss; danger medium.

## Do

Named, persistent storage that survives the container it mounts into. `list` is
the default; the rest are callable by name. A volume is attached at run time with
`-v name:/path` ([[docker-cli]]) on [[docker-container]].

`rm` is destructive — it permanently deletes the volume's data and only succeeds
when no container references it. To sweep every unused volume at once, see
[[docker-prune]].

| Recipe | Maps to | Notes |
|--------|---------|-------|
| `list` | `docker volume ls` | all named volumes |
| `create` | `docker volume create` | make a named volume |
| `rm` | `docker volume rm` | delete a volume and its data |
| `inspect` | `docker volume inspect` | mountpoint + driver as JSON |

```just
# list all named volumes (default)
list:
  docker volume ls

# create a named volume
create name:
  docker volume create -- {{quote(name)}}

# remove a volume and everything stored in it
rm name:
  docker volume rm -- {{quote(name)}}

# show a volume's mountpoint and driver as JSON
inspect name:
  docker volume inspect -- {{quote(name)}}
```

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `list` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
