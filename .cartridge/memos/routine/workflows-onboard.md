---
kind: routine
description: Set up a fresh clone for development end to end. Use when the user says "set this up", "onboard",
  or "get me running".
uses:
- usage: '[[run-usage]]'
  when:
  - onboard a clone
  - get me running
  - set up the project
  - bootstrap a checkout
  tags:
  - workflow
  - setup
  - onboard
  - dev
---

## Inputs

No prerequisites beyond the commands named below. Recipe parameters are named in Do; supply paths and arguments for the current task.

## Do

The steps a new contributor runs once: install, prepare env, then `migrate seed`
from [[tools-db]] (the DB needs the env in place first). Does not run the gate. For
ongoing migrations use [[tools-db]] directly.

```just
# install + env first, copy .env, then migrate + seed the DB last
onboard: install env && migrate seed
  cp .env.example .env

install:
  npm ci

env:
  @test -f .env.example
```

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `onboard` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
