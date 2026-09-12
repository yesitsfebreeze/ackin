---
kind: routine
description: Build and run the project in Docker (build, up, down, logs, shell). Use when the user asks
  to containerize, run locally, or debug the container.
uses:
- usage: '[[run-usage]]'
  when:
  - containerize the app
  - run in a container
  - build an image
  - dockerize
  tags:
  - docker
  - container
  - dev
---

## Inputs

Requires on PATH: `docker`. Recipe parameters are named in Do; supply paths and arguments for the current task.
Risk: side effects network; danger low.

## Do

Local container lifecycle. `up` is the default; `build`/`down`/`logs`/`shell`
are callable by name. Real config lives in `compose.yaml` / `Dockerfile` on
disk; these recipes are thin entry points.

```just
# build images without starting anything
build service="":
  docker compose build {{service}}

# bring the stack up in the background (default)
up:
  docker compose up -d

# tear the stack down
down:
  docker compose down

# tail logs, optionally for one service
logs service="":
  docker compose logs -f {{service}}

# open a shell inside a running service
shell service="app":
  docker compose exec {{service}} sh
```

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `up` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
