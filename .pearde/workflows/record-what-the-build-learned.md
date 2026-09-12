---
atomic: record-what-the-build-learned
subject: the split and the dead workarounds are one worker's head until they are written to the record
date: 2026-09-12
updated: 2026-09-12
runs: 4
tags:
  - atomic
---

## Do

1. Pipe the finding into `python3 resources/knowledge.py remember "<title>"` with `--provenance` naming the route or measurement it came from.
2. Record the negative results too — the workarounds that did not work are what the next worker would otherwise pay for again.

## Done when

- Every fact the report states that the next worker would have to rediscover has a note id, and the report cites it.

## Fails when

- Only the things that worked are written down. A ruled-out workaround is the more expensive half of the finding, because the next pass will otherwise try it again.
- The note is left in the lane, where the worktree takes it.
