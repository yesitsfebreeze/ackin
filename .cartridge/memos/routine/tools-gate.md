---
kind: routine
description: Run the project's build/test gate. Use before any release or merge to check the tree is green.
uses:
- usage: '[[run-usage]]'
  when:
  - run tests
  - build check
  - pre-merge gate
  - is the tree green
  tags:
  - ci
  - gate
  - test
  - build
  - go
---

## Inputs

Requires on PATH: `go`. Recipe parameters are named in Do; supply paths and arguments for the current task.

## Do

Builds every package exactly as the release CI does and runs the tests; exits
non-zero on any failure. Release routines depend on `gate`,
so a red gate aborts them before anything is tagged or pushed.

```just
# full gate: build + tests
gate:
  go build ./...
  go test ./...
```

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `gate` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
