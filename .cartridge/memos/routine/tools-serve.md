---
kind: routine
description: Start the dev server (Vite) and keep it running. Use when the user asks to run, start, or
  serve the app locally.
uses:
- usage: '[[run-usage]]'
  when:
  - run the app
  - launch locally
  - dev server
  - serve locally
  - start the frontend
  not_when:
  - production
  - deploy
  - build for release
  tags:
  - dev
  - server
  - vite
  - sidecar
---

## Inputs

Requires on PATH: `node`. Recipe parameters are named in Do; supply paths and arguments for the current task.
Risk: side effects network.

## Do

Long-running sidecar: prints `READY <url>` once Vite is listening, then stays
alive. One-shot recipes in the same file: `reload`, `status`, `stop`.

```just
# long-running dev server; prints READY once Vite is listening
serve port="3000":
  @echo "READY http://localhost:{{port}}"
  vite --port {{port}} --host

# one-shot: nudge Vite to reload
reload:
  @touch vite.config.ts

# one-shot: is the server up? non-zero exit if down
status port="3000":
  @curl -fsS http://localhost:{{port}}/health && echo " up" || (echo "down" && exit 1)

# one-shot: stop by pidfile (the runner may also SIGTERM the process)
stop:
  @test -f .vite.pid && kill $$(cat .vite.pid) && rm .vite.pid || echo "not running"
```

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `serve` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
