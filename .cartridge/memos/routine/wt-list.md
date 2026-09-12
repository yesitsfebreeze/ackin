---
kind: routine
description: Show every worktrunk worktree in one table — branch, working-tree status, divergence from
  default and remote, commit age and message, optionally CI status and LLM summaries. Use to see what
  is in flight across all your parallel branches.
uses:
- usage: '[[run-usage]]'
  when:
  - list worktrees
  - see all branches in flight
  - worktree status overview
  - check CI per worktree
  - json list of worktrees
  tags:
  - worktrunk
  - wt
  - list
  - worktree
  - status
  - overview
  - vcs
---

## Inputs

Requires on PATH: `wt`. Recipe parameters are named in Do; supply paths and arguments for the current task.
Risk: danger low.

## Do

A dashboard of every worktree: branch, path, status symbols for uncommitted
changes, ahead/behind vs the default branch and the remote, plus commit age and
message. The `main…±` divergence is local git, so it shows by default with no
network.

| Recipe | Flags | Does |
|--------|-------|------|
| `list` | — | compact table from local git only (fast, offline) |
| `full` | `--full` | adds CI status + LLM branch summaries (network) |
| `json` | `--format json` | machine-readable for scripting |

`--full` is the network-heavy view — CI pipeline results and AI-generated branch
summaries — keep it off for a quick glance. `--branches` adds branches without a
worktree; `--remotes` adds remote-only branches. Columns and timeouts are tunable
in [[wt-config]] (`[list]`), including custom template columns.

```just
# compact table: branch, status, divergence, age, message (offline)
list:
  wt list

# add CI status and LLM branch summaries (network)
full:
  wt list --full

# machine-readable output
json:
  wt list --format json
```

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `list` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
