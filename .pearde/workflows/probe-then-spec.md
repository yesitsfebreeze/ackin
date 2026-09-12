---
workflow: probe-then-spec
subject: The host binary
date: 2026-09-12
updated: 2026-09-12
runs: 12
tags:
  - workflow
---

## Use when

- A PRD came back from a question with its `## Answers` filled and pass one's probe already standing uncommitted in the tree, and the answer has to be turned into specs.
- Not when the PRD has never been probed and has no answers — that is the first pass of this same route, which stops at the question instead of continuing past it.

## Steps

| # | atomic | why | on failure |
|---|--------|-----|------------|
| 1 | `read-the-contract` | pass one's report holds the call-site census this pass edits from, and the answer under `## Answers` closes the only fork left | `stop` |
| 2 | `query-the-record-first` | the machine stall this run hits was already on record from pass one, so no time was spent rediscovering it | `→ 1` |
| 3 | `apply-the-answered-fork` | the answer is a sentence until the module is actually split; the split is what shows which callers die with the cut part | `→ 1` |
| 4 | `port-the-tests-the-cut-orphaned` | every test of the cut part asserted through it, so deleting the file silently drops coverage of the part that was kept | `→ 3` |
| 5 | `attempt-the-build` | a rename applied by string match across eight files is a guess until a compiler reads it | `→ 3` |
| 6 | `separate-the-machine-failure-from-the-crate` | a build that produces no output looks identical to a build that failed, and specs written on that confusion blame the wrong thing | `→ 5` |
| 7 | `record-what-the-build-learned` | the split and the dead workarounds are one worker's head until they are written to the record | `→ 3` |
| 8 | `write-the-specs` | the next worker gets the file, not the head, and every unverified claim has to arrive as a box that can fail | `→ 3` |
