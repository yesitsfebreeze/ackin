---
kind: principle
description: Apply when two or more fixes sharing one premise have failed the same gate. Take a census of which actors hold the imbalance before the next fix, then question the premise instead of writing another fix that assumes it.
group: core
---

# Attack the premise

When two or more fixes that share one premise have failed the same gate,
suspect the premise, not the fixes.

**Why:** Each failure under a shared premise is evidence about the premise.

**The pattern:**

- **Write the premise down.** One sentence, the thing every failed fix assumed.
- **Take a census before the next fix.** Count the imbalance per actor. The
  census shows which actors hold the imbalance, not how large it is. Write it
  as a rerunnable script, per [[build-the-lever]].
- **Read the skew.** If the same few actors hold most of the imbalance on
  every run, something assigns them that role. Find what assigns it; that
  assignment is the next "why", per [[fix-root-causes]].
- **Remove the asymmetry instead of compensating for it**, per
  [[laziness-protocol]]. Rotate the role, randomize the assignment, or move
  it, so no actor holds it every run. A return path, a shared pool, a batched
  hand-off or a periodic rebalance leaves the assignment in place and adds
  work on every run.

**Stop:**

- Do not start the next fix before the premise is written down and the census
  exists.
- If the census is even across actors, the premise is not the cause. Look
  elsewhere and keep the census as evidence.

Distinct from [[redesign-from-first-principles]], which rebuilds a design
around a new requirement. This questions a fact the current design assumes.
