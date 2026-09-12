---
atomic: write-the-specs
subject: the next worker gets the file, not the head, and every unverified claim has to arrive as a box that can fail
date: 2026-09-12
updated: 2026-09-12
runs: 0
tags:
  - atomic
---

## Do

1. Split what the build stands up into implementable units and write each to `specs/specNN.md` from the template.
2. Give each a `footprint:` of the paths it writes — never a root that clashes with the board — and a `complexity:`.
3. Under each, say what already stands in the tree and what is left to finish.
4. Make the first box of every spec the check that was never run, and give each spec a verify command scoped to its footprint.

## Done when

- Every claim the probe made is a box that a command can fail, the footprints cover the tree the probe moved and nothing else, and the summed complexity and the spec count are inside the board's ceilings.

## Fails when

- An unverified claim arrives as prose rather than as a box that can fail, so nothing downstream can ever catch it being wrong.
