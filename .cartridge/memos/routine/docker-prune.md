---
kind: routine
description: Reclaim disk by pruning unused docker objects — the whole system, or just stopped containers,
  dangling images, unused volumes, or the build cache. Use to free space, but only when the removed objects
  are truly disposable.
uses:
- usage: '[[run-usage]]'
  when:
  - reclaim docker disk space
  - prune docker
  - remove unused images
  - clean up stopped containers
  - prune volumes
  - clear the build cache
  - docker system prune
  tags:
  - docker
  - prune
  - cleanup
  - disk
---

## Inputs

Requires on PATH: `docker`. Recipe parameters are named in Do; supply paths and arguments for the current task.
Risk: side effects disk, data-loss; danger high.

## Do

Bulk deletion of unused objects to reclaim disk. `system` is the default and the
broadest; the rest scope to one object type. Every recipe here is irreversible —
it removes whatever is not currently in use, with no per-object confirmation.

`system` with `-a` is the most aggressive: it drops stopped containers, unused
networks, dangling **and** unreferenced images, and the build cache. `volumes`
can erase data a stopped service still needs ([[docker-volume]]), so it is excluded
from `system` by default — run it deliberately. The targeted variants narrow the
blast radius: prefer them over `system` when you know what you want gone. Single
objects are removed in [[docker-container]], [[docker-image]], [[docker-volume]], and
[[docker-network]].

| Recipe | Maps to | Removes |
|--------|---------|---------|
| `system` | `docker system prune -a -f` | stopped containers, unused nets, all unused images, build cache |
| `containers` | `docker container prune -f` | every stopped container |
| `images` | `docker image prune -a -f` | every image no container uses |
| `volumes` | `docker volume prune -f` | every volume no container uses (data loss) |
| `builder` | `docker builder prune -a -f` | the entire build cache |

```just
# reclaim everything unused: stopped containers, nets, all unused images, cache (default)
system:
  docker system prune -a -f

# remove every stopped container
containers:
  docker container prune -f

# remove every image no container references
images:
  docker image prune -a -f

# remove every volume no container references (destroys their data)
volumes:
  docker volume prune -f

# clear the entire build cache
builder:
  docker builder prune -a -f
```

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `system` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
