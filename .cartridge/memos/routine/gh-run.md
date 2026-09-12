---
kind: routine
description: Observe and control GitHub Actions runs — list, view, watch, rerun, logs. Use to check CI
  status, follow a run live, read failure logs, or re-trigger a failed run.
uses:
- usage: '[[run-usage]]'
  when:
  - check ci
  - github actions status
  - watch a workflow run
  - rerun a failed run
  - view run logs
  - why did ci fail
  tags:
  - gh
  - github
  - actions
  - ci
  - run
---

## Inputs

Requires on PATH: `gh`. Recipe parameters are named in Do; supply paths and arguments for the current task.
Risk: side effects network; danger none.

## Do

CI observation surface for the cwd's repo (retarget via `-R`, [[gh-cli]]). `list` is
the default; the rest are callable by name. `list`/`view`/`watch`/`logs` only read
— `rerun` re-triggers a run (the one action here).

| Recipe | Maps to | Notes |
|--------|---------|-------|
| `list` | `gh run list` | recent runs; `limit` caps the count |
| `view` | `gh run view` | status + job tree for one run |
| `watch` | `gh run watch` | follow a run live until it ends |
| `rerun` | `gh run rerun` | re-trigger; `failed=true` reruns only failed jobs |
| `logs` | `gh run view --log` | dump logs (pair with `--jq`/grep to find the failure) |

```just
# list recent runs (default 20)
list limit="20":
  gh run list --limit {{quote(limit)}}

# view one run's status and jobs (blank = pick interactively)
view run_id="":
  gh run view {{quote(run_id)}}

# follow a run live until it completes
watch run_id="":
  gh run watch {{quote(run_id)}}

# re-trigger a run; failed=true reruns only the failed jobs
rerun run_id failed="false":
  #!/usr/bin/env sh
  set -eu
  r={{quote(run_id)}}; f={{quote(failed)}}
  set -- "$r"
  [ "$f" = "true" ] && set -- "$@" --failed
  gh run rerun "$@"

# dump a run's logs
logs run_id:
  gh run view {{quote(run_id)}} --log
```

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `list` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
