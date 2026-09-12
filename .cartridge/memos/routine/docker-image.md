---
kind: routine
description: The image lifecycle — build from a Dockerfile, pull, push, tag, list, and remove images.
  Use to build an image, fetch one from a registry, retag it, publish it, or clean images off disk.
uses:
- usage: '[[run-usage]]'
  when:
  - build a docker image
  - build from a dockerfile
  - pull an image
  - push an image to a registry
  - tag an image
  - list images
  - remove an image
  - publish an image
  tags:
  - docker
  - image
  - build
  - registry
  - tag
---

## Inputs

Requires on PATH: `docker`. Recipe parameters are named in Do; supply paths and arguments for the current task.
Risk: side effects network, disk; danger medium.

## Do

The build-and-distribute side of docker. `build` is the default; the rest are
callable by name. An image is what a container runs ([[docker-container]]); flag and
targeting conventions come from [[docker-cli]]. Compose builds images too — see
[[tools-docker]] for a stack-level build.

`push` touches a registry (`network`); `rm` is destructive and frees disk. To
reclaim space from dangling layers in bulk, see [[docker-prune]].

| Recipe | Maps to | Notes |
|--------|---------|-------|
| `build` | `docker build -t` | `tag` the image; `context` defaults to `.` |
| `pull` | `docker pull` | fetch `ref` from its registry |
| `push` | `docker push` | publish `ref` to its registry |
| `tag` | `docker tag` | add a second name to an existing image |
| `list` | `docker images` | all local images |
| `rm` | `docker rmi` | remove an image by name or id |

```just
# build an image and tag it; context defaults to the cwd
build tag context=".":
  docker build -t {{quote(tag)}} -- {{quote(context)}}

# pull an image reference from its registry
pull ref:
  docker pull -- {{quote(ref)}}

# push an image reference to its registry
push ref:
  docker push -- {{quote(ref)}}

# add a new tag to an existing image (src -> dst)
tag src dst:
  docker tag {{quote(src)}} {{quote(dst)}}

# list all local images (default)
list:
  docker images

# remove a local image by name or id
rm ref:
  docker rmi -- {{quote(ref)}}
```

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `build` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
