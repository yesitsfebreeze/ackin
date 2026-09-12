---
kind: routine
description: Deliver one specced work memo's approach and prove it against its own checks
uses:
  - usage: "[[run-usage]]"
    when: [a work memo carries an Approach and is ready to build, delivering a bounded outcome]
    tags: [implementation, work]
---

## Inputs

One work memo carrying `## Outcome`, `## Approach` and `## Check`, given by
name. The Approach is the whole plan. Nothing else is assigned.

## Do

Read the memo: the Outcome for why, the Approach for what to run, the Check for
what must hold.

Open `just lane <memo-name>` from the workspace root and `cd` into it. The lane
already holds the attempt made while the memo was specced — continue it rather
than restarting.

Run the Approach step by step with the tools it names. Nothing beyond it,
nothing half of it. A step that does not work as written is not reinterpreted;
see Failure. Commit each smallest coherent change as it becomes reviewable.

Tick each `- [ ]` in `## Check` to `[x]` as that check passes, quoting the
passing output in the memo itself, as it closes rather than in a batch at the
end: the boxes are the only live view of the run. A box ticked without its
quoted evidence counts as unmet, not done. Run [[repository-checks]] last.
Never tick a box that was not observed passing. An unexpected failure stops the work there,
and a check that says it is repaired counts only once it was seen failing on the
same build.

Return one of three results, written into the memo through the memo tool:

- **Done** — every box is `[x]`, `status: done`, and `## Result` records the
  delivered behaviour and the evidence for it.
- **Blocked** — a wall no worker can pass: missing configuration, a surface only
  a person can operate. `status: blocked` and `## Blocker` naming the obstacle
  and what clears it.
- **Failed** — the Approach does not work as written, or a box will not close.
  The memo stays open, the failure is written into its body as the first thing
  the next pass must answer, and the lane keeps its commits for the retry.

## Check

The memo carries one of those three results through a validated write. On Done,
every Check box is ticked, `## Result` exists, [[repository-checks]] passed, and
the work is committed on the lane branch. A clean worktree on its own proves
nothing.

## Failure

Never widen the Approach and never split: a contract that does not fit is
Failed with the reason, and it is re-specced by [[spec-a-work-memo]]. A defect
found outside this memo's scope is a new work memo with an Outcome and no
Approach, never a fix in the wrong lane. Report any check that failed on its
first run even when a focused retry passes.
