---
atomic: port-the-tests-the-cut-orphaned
subject: every test of the cut part asserted through it, so deleting the file silently drops coverage of the part that was kept
date: 2026-09-12
updated: 2026-09-12
runs: 2
tags:
  - atomic
---

## Do

1. Read every test of the module being cut and mark which guarantee each one actually asserts.
2. Rewrite the ones asserting a kept guarantee so they assert it through what remains, in a file named for the kept half.
3. Delete the ones that only asserted the cut half, and update any fixture whose config existed to drive it.

## Done when

- Each guarantee the contract keeps has a test that names it, and `grep` for the cut half's vocabulary across the test tree returns nothing.

## Fails when

- The test file of the cut part is deleted whole, taking with it the assertions that covered the part that was kept.
