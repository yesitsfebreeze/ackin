---
atomic: separate-the-machine-failure-from-the-crate
subject: a build that produces no output looks identical to a build that failed, and specs written on that confusion blame the wrong thing
date: 2026-09-12
updated: 2026-09-12
runs: 1
tags:
  - atomic
---

## Do

0. This atomic is entered only when the build stalls with no diagnostic. If
   the build printed its success line, record the step as not applicable,
   quote that line, and go on — there is no machine failure to separate from
   the crate. A machine that recovered between passes is the common case on a
   re-run, and it is not a failure of this atomic.
1. When the build stalls with no diagnostic, check whether the processes are burning CPU or parked at 0%.
2. Reduce it to the smallest thing outside the project that shows the same symptom, and keep that reproduction as a script in `probe/`.
3. Re-test the workarounds a previous pass left untried, and write down the ones that do not work as well as the ones that do, and stop when the remaining candidates all require a privilege you do not have; say so and name the privilege rather than continuing.
4. Say in the report, and in the first acceptance box of every spec, that the tree is unverified and what will verify it.

## Done when

- The failure is demonstrated outside the project under build, the reproduction is runnable from `probe/`, and no spec claims a check that was never run.

## Fails when

- The workaround list is worked without a stopping rule. Workaround space is unbounded; the stop is the first privilege the worker does not hold, named out loud.
- The reproduction only exists inside the project's own build, so it cannot tell a broken machine from a broken crate — which is the one thing this atomic is for.
