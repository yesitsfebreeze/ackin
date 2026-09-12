---
kind: routine
description: Fan out N parallel workers over slices or races, drain them, and return one consolidated report with a result table and evidenced issues.
uses:
  - usage: "[[run-usage]]"
    when: [running parallel coverage over many slices, racing several workers on one brief, partitioning an exploration]
---

Fan out N parallel workers. They may cover separate slices, race the same
brief, or mix both. You wait, aggregate, and return one report. Use [[arena]]
instead when the workers compete on one artifact and the output is a synthesis
rather than a report.

1. Frame

- State the done predicate and the artifact or report the swarm must return.
- Choose the shape: partition into slices, race N workers on identical briefs,
  or mix. For a race or mixed shape, declare `first pass`, `rank all` or
  `best-of` before spawning, never after seeing the results.
- Set N from the caller or derive it from the shape. N is total workers, not a
  concurrency limit.
- For a model race, name each arm's model up front.
- Give each worker its own writable output, per
  [[separate-before-serializing-shared-state]].

2. Fan out

Dispatch all N in one message, in the background. Every brief stands alone:
the goal, the scope, its exact slice or race arm, how to verify, and what to
report. Reports use `PASS`, `ISSUES` or `BLOCKED` with evidence.

If a worker drops out, proceed with N-1 and note it.

3. Aggregate

Read the terminal results. For coverage, every required slice needs a result.
For a race, apply the selection rule declared in step 1.

Do not paste raw worker dumps — that is what [[guard-the-context-window]]
forbids. Keep a compact result table, one-line evidenced issues, and explicit
gaps or dropouts.

4. Report

One consolidated report: the table, the issue one-liners, the gaps and
dropouts, and the race rule when one was used.

Check: every required slice has a row, or an explicit dropout line. A worker's
`PASS` with no evidence is an `ISSUES` until it produces one, per
[[prove-it-works]].

Failure: the results disagree about the same fact. Do not average them. Read
the contested thing yourself and report what you observed.
