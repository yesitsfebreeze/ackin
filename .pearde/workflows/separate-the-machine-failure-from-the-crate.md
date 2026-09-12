---
atomic: separate-the-machine-failure-from-the-crate
subject: a build that produces no output looks identical to a build that failed, and specs written on that confusion blame the wrong thing
date: 2026-09-12
runs: 0
tags:
  - atomic
---

## Do

1. When the build stalls with no diagnostic, check whether the processes are burning CPU or parked at 0%.
2. Reduce it to the smallest thing outside the project that shows the same symptom, and keep that reproduction as a script in `probe/`.
3. Re-test the workarounds a previous pass left untried, and write down the ones that do not work as well as the ones that do.
4. Say in the report, and in the first acceptance box of every spec, that the tree is unverified and what will verify it.

## Done when

- The failure is demonstrated outside the project under build, the reproduction is runnable from `probe/`, and no spec claims a check that was never run.

## Fails when
