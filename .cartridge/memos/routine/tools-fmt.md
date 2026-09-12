---
kind: routine
description: Format source files, or check formatting without writing. Use when the user asks to pretty-print
  or verify style.
uses:
- usage: '[[run-usage]]'
  when:
  - format the code
  - pretty-print
  - check style
  - auto-format
  tags:
  - format
  - style
  - lint
---

## Inputs

No prerequisites beyond the commands named below. Recipe parameters are named in Do; supply paths and arguments for the current task.

## Do

`fmt` is the default and writes formatted files; `check` is the non-mutating
variant (exits non-zero if any file would change) for CI and pre-commit hooks.
Both take an optional `path` (default `.`).

```just
# write formatted files (default)
fmt path=".":
  npx prettier --write {{path}}

# non-mutating check: exits non-zero if any file would change
check path=".":
  npx prettier --check {{path}}
```

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `fmt` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
