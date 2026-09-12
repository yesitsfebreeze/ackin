---
kind: routine
description: Spawn N parallel candidates at one task, read every one, pick a base, graft the strongest parts of the losers into it, and verify the synthesis.
uses:
  - usage: "[[run-usage]]"
    when: [one attempt at a non-trivial artifact would lock in the wrong shape, running a design or code bakeoff]
---

Fan out N parallel attempts at the same task. Read every candidate end to end.
Pick the strongest as the base. Graft the best ideas from the others into it.
Verify the result. Use [[swarm]] instead when the workers cover separate
slices rather than competing on one.

1. Frame

The N candidates receive the same prompt, so the prompt is the contract.

- State the artifact each candidate produces.
- Derive the rubric. Say what success looks like for this task, then turn it
  into three to six concrete gradeable criteria. The rubric is the picker's
  tool in step 4; candidates see only the task.
- Pick the runners. Prefer different model families when the work is
  judgment-sensitive; the same model N times is fine when the work is
  generation-bound. Spawn more when the arena covers several design
  directions.
- Assign output paths. Each candidate writes to its own location — a worktree
  where possible, otherwise its own directory — per
  [[separate-before-serializing-shared-state]].

2. Fan out

Dispatch all N in one message, each with the task, the path to the shared
grounding, its own output path, and instructions to produce both the artifact
and a short rationale naming the alternatives it considered and rejected.

If a candidate produces nothing, proceed with N-1 and note the dropout.

3. Cross-judge

After every candidate completes, dispatch one read-only judge, preferably on a
different model family from your own. It sees the rubric and the candidates by
path label, scores each criterion, and recommends a base with rationale. It
runs in parallel with your own reading in step 4, never while candidates are
still writing.

4. Pick a base

Read every candidate end to end before picking. Score criterion by criterion,
not on holistic feel.

Compare against the cross-judge. Agreement confirms the pick. Disagreement
means one of you is biased or the rubric was ambiguous — read both rationales
before deciding.

Pick the base a future maintainer can extend most easily without breaking
invariants. Prefer the cleaner boundary or smaller API when two feel tied, per
[[laziness-protocol]].

Record the pick and the reason alongside the base artifact, including the
cross-judge's verdict.

5. Graft

Walk each losing candidate once more for what is worth porting. The signal is
usually one or two things per candidate, not most of it.

Fold each graft in by hand, per [[redesign-from-first-principles]]. Do not
paste mechanically — the result must stay coherent under one mental model.

Record what was grafted, from which candidate, and what was rejected and why.

When candidates converge on the same shape, that is a strong agreement signal:
note the convergence and ship the consensus shape, no graft needed. When they
wildly diverge, step 1 was under-specified. Reframe and rerun rather than
averaging the divergence.

6. Verify

The synthesized artifact holds up under the same scrutiny as any other output,
per [[prove-it-works]].

Check: one synthesized artifact, plus one note naming the base, the grafts
with their source candidate, the rejections, any dropouts, and the
verification result.

Failure: verification surfaces a problem the arena did not catch. Either step
1 was wrong, so reframe and rerun, or one candidate caught it and the graft
was missed, so return to step 5. Do not paper over it.
