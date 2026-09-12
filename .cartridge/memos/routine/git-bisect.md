---
kind: routine
description: Binary-search the history to find the commit that introduced a regression. Use to locate
  exactly when a bug or behavior change first appeared.
uses:
- usage: '[[run-usage]]'
  when:
  - find the commit that broke it
  - bisect a regression
  - when did this break
  - binary search history
  - locate a bad commit
  tags:
  - git
  - bisect
  - debug
  - regression
  - vcs
---

## Inputs

Requires on PATH: `git`. Recipe parameters are named in Do; supply paths and arguments for the current task.
Risk: side effects git-write; danger low.

## Do

Binary search over a commit range to pin the first bad commit. Mark a known-bad
and known-good endpoint (any commit-ish, see [[git-refs]]); git checks out the
midpoint, you mark it `good`/`bad`, and it halves the range until one commit
remains. `run` automates this with a test script that exits 0 for good, non-zero
for bad. `skip` sidesteps an untestable commit; `reset` ends the session and
returns to the original HEAD. Inspect candidates with [[git-history]].

| Recipe | Does |
|--------|------|
| `start bad good` | begin a bisect between two endpoints (default) |
| `good` | mark the current checkout as good |
| `bad` | mark the current checkout as bad |
| `skip` | skip an untestable commit |
| `run cmd` | auto-bisect: run `cmd` at each step (0=good) |
| `reset` | end the session, return to original HEAD |

```just
# begin a bisect: bad endpoint then good endpoint (default)
start bad good:
  git bisect start {{quote(bad)}} {{quote(good)}}

# mark the current checkout good
good:
  git bisect good

# mark the current checkout bad
bad:
  git bisect bad

# skip an untestable commit
skip:
  git bisect skip

# auto-bisect by running a test command at each step (exit 0 = good)
run +cmd:
  git bisect run {{cmd}}

# end the session and return to the original HEAD
reset:
  git bisect reset
```

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `start` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
