---
atomic: read-the-contract
subject: pass one's report holds the call-site census this pass edits from, and the answer under `## Answers` closes the only fork left
date: 2026-09-12
updated: 2026-09-12
runs: 2
tags:
  - atomic
---

## Do

1. Run `pearde brief <prd> --worker <you>` and read nothing it does not name.
2. Read `.pearde/prds/<prd>/prd.md` including `## Questions` and `## Answers`.
3. Read `.pearde/prds/<prd>/report.md` and `.pearde/prds/<prd>/probe/README.md` from the prior pass, and inspect the uncommitted tree they describe.

## Done when

- The answered fork is stated in one sentence, and the prior pass's call-site census has been checked against the tree with `grep` rather than trusted.

## Fails when

- A census or a count from an earlier pass is carried forward untested. The contract is re-read against the tree, not against the last report.
