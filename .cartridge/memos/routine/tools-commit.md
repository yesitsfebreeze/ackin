---
kind: routine
description: Stage a scope and write a conventional commit. Use when the user asks to commit changes with
  a message.
uses:
- usage: '[[run-usage]]'
  when:
  - commit changes
  - write a commit message
  - stage and commit
  - conventional commit
  tags:
  - git
  - commit
  - vcs
---

## Inputs

Requires on PATH: `git`. Recipe parameters are named in Do; supply paths and arguments for the current task.
Risk: side effects git-write; danger low.

## Do

[Conventional commit](https://www.conventionalcommits.org) message; prints the new short SHA.
Defaults: `type=feat scope=. message=update`. Args map positionally: `type scope message`.

```just
commit type="feat" scope="." message="update":
  git add {{scope}}
  git commit -m "{{type}}({{scope}}): {{message}}"
  git rev-parse --short HEAD
```

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `commit` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
