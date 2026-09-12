---
kind: routine
description: Run multi-container apps with Docker Compose — bring the stack up or down, follow service
  logs, and list running services from a compose file. Use to start a defined stack, tear it down, or
  watch what a service is doing.
uses:
- usage: '[[run-usage]]'
  when:
  - start a docker compose stack
  - bring up the services
  - tear down containers
  - follow compose logs
  - list compose services
  - docker compose up
  - restart a stack
  tags:
  - docker
  - compose
  - stack
  - services
  - orchestration
  - multi-container
---

## Inputs

Requires on PATH: `docker`. Recipe parameters are named in Do; supply paths and arguments for the current task.
Risk: side effects process, network; danger medium.

## Do

Drive a whole stack defined in `compose.yaml` rather than single containers
([[docker-container]]). `up` starts every service (detached); `down` stops and
removes them; `logs` follows the combined output; `ps` lists what is running.
Modern Docker ships this as `docker compose` (a subcommand); the standalone
`docker-compose` is the legacy v1 binary with the same verbs.

| Recipe | Does |
|--------|------|
| `up` | build (if needed) and start the stack, detached |
| `down` | stop and remove the stack's containers and network |
| `logs` | follow the combined logs of all (or one) service |
| `ps` | list the stack's services and their state |

`up -d` runs detached; `up --build` forces a rebuild when an image is stale;
`down -v` also removes named volumes (data loss — deliberate). Pass a service
name to `logs`/`up`/`down` to scope to one. `down` only removes what *this*
compose project created, so it is safe next to unrelated containers. Edit the
[[search-yq]] way for compose-file tweaks.

```just
# build if needed and start the whole stack, detached (default)
up service="":
  docker compose up -d {{service}}

# stop and remove the stack's containers and network
down:
  docker compose down

# follow the combined logs (pass a service name to scope)
logs service="":
  docker compose logs -f {{service}}

# list the stack's services and their state
ps:
  docker compose ps
```

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `up` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
