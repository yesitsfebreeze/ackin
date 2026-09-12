---
kind: routine
description: Open files side by side in vim diff mode (vimdiff), including a three-way merge or a file
  against its last commit. Use to review or resolve differences in vim.
uses:
- usage: '[[run-usage]]'
  when:
  - diff two files in vim
  - vimdiff
  - three-way merge
  - review changes in vim
  - compare files
  tags:
  - vim
  - diff
  - vimdiff
  - merge
  - review
---

## Inputs

Requires on PATH: `nvim`. Recipe parameters are named in Do; supply paths and arguments for the current task.
Risk: danger low.

## Do

`nvim -d` aligned side by side, scrollbound, with `]c`/`[c` to jump hunks and
`do`/`dp` to obtain/put a change (see [[vim-windows]] for moving between the
windows). `diff` compares two files; `three` is a 3-way merge; `git` compares a
working-tree file to its committed version.

```just
# open two files in diff mode (default)
diff a b:
  nvim -d -- {{quote(a)}} {{quote(b)}}

# three-way diff (base + two sides)
three base a b:
  nvim -d -- {{quote(base)}} {{quote(a)}} {{quote(b)}}

# diff a working-tree file against its last committed version
git file:
  #!/usr/bin/env sh
  set -eu
  f={{quote(file)}}
  tmp=$(mktemp); trap 'rm -f "$tmp"' EXIT
  git show "HEAD:./$f" > "$tmp" 2>/dev/null || true
  nvim -d -- "$tmp" "$f"
```

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `diff` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
