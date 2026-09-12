---
atomic: apply-the-answered-fork
subject: the answer is a sentence until the module is actually split; the split is what shows which callers die with the cut part
date: 2026-09-12
runs: 0
tags:
  - atomic
---

## Do

1. Write the kept half into its own module and delete the cut half outright.
2. `grep -rn` every symbol of the cut half across `src` and the tests, and remove each caller.
3. Where a kept operation was only meaningful because of the cut half, collapse it to what remains rather than porting it.
4. `grep` for the cut names once more; an empty result is the check.

## Done when

- No symbol of the cut half survives anywhere in the tree, and the kept half compiles as a module that stores nothing the cut half stored.

## Fails when
