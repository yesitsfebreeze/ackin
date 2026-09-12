---
kind: routine
description: "Land one worked lane - re-verify its checks on the combined result, then fast-forward"
uses:
  - usage: "[[run-usage]]"
    when: [a lane's work memo reports Done and the coordinator collects it]
    tags: [landing, work]
---

## Inputs

One lane whose work memo reports Done, and the trunk to land it on.

## Do

Landing is the Check on the combined result, not extra diligence:

1. Re-read the memo's `## Check` and run its `sh` block (and [[repository-checks]])
   on the lane rebased onto the current trunk — the combined result, not the
   lane alone. A box that passed before the rebase proves nothing about the
   combination.
2. Rebase the lane onto the trunk; resolve by preserving both sides.
3. Commit the memo's own record state (tick marks, Result, claim strikes) that
   belongs to this lane.
4. `just land <name>` to fast-forward the trunk onto the lane branch, then
   `just lane-rm <name>` when the worktree holds nothing unmerged.
5. Strike the memo's `claim:` line; re-read the board and dispatch what the
   landing unblocked.

## Check

The landed trunk contains the lane's commits and the memo's evidence; the
re-run `sh` block passed on the combined result; the lane is closed. A
refusing `land` names a trunk file somebody left dirty — that is theirs to
lane, not this routine's to resolve.

## Failure

A failed re-verification keeps the lane and worktree with the remaining work
recorded in the memo. Never land around a red check, and never reuse a
finished lane for unrelated work.
