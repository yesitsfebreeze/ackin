---
kind: principle
description: Apply when designing commands, lifecycle steps, or processing loops that run amid crashes, restarts, and retries. Converge to the same end state regardless of partial prior runs.
group: architecture
---

# Make operations idempotent

Design operations so they converge to the correct state regardless of how many
times they run or where they start from. Every state-mutating operation
answers two questions: what happens if this runs twice, and what happens if
the previous run crashed halfway.

**Why:** Commands, lifecycle operations and processing loops run where
crashes, restarts and retries are normal. If partial state changes the next
run's outcome, every restart becomes a debugging session.

**The pattern:**

- Convergent startup: scan for existing state, clean stale artifacts, adopt
  live sessions.
- Content-based cleanup: compare by content equivalence, not creation order.
- Self-healing locks: detect a stale lock by whether its owner still lives.
- Idempotent scheduling: failed work respawns cleanly, fresh input is
  regenerated after each cycle.

**The test:**

1. What happens if this runs twice in a row?
2. What happens if the previous run crashed at every possible point?
3. Does re-execution converge to the same end state?

If any answer is "it depends what state was left behind", the operation needs
a reconciliation step.

The debugging corollary is in [[fix-root-causes]]: something that fails after
restart is stale state until proven otherwise.
