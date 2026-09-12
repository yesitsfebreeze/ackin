---
kind: principle
description: Apply to any non-trivial work — edits, migrations, analyses, checks. Build the tool that does it or proves it instead of working by hand; the tool is the artifact a reviewer can rerun.
group: core
---

# Build the lever

When the work is not trivial, build the tool that does it instead of doing it
by hand.

**Why:** Two payoffs. Throughput — a codemod, generator or script does the
work the same way every time and reruns for free. Confidence — the tool is one
artifact a reviewer can read and rerun to check the work. Hand-done changes
can only be re-verified by redoing them. A deterministic script turns "trust
me" into "run this".

**The pattern:** default to building the lever. Skip it only when the task is
trivial, a couple of obvious edits visible at a glance.

- Do the first unit by hand to learn the recipe, then build the tool. Prove it
  by rerunning it on that unit and diffing against the hand-done version. Make
  the lever safe to rerun.
- Codemod or script for edits, generator for repetitive files, a query for
  analysis, a rerunnable check for verification.
- A deterministic lever beats fan-out. If the tool can process every unit in
  one pass, run it. Do not fan out delegates to hand-apply what a script does.
- When you do fan work out, write the lever as one artifact every delegate
  reads: the recipe, the verification contract and the do-not-touch fences.
  Keep it outside their write scope so they cannot quietly edit the contract.
- Applying this principle produces a file. If you cited it and there is no
  script, codemod, generator or delegate brief in the diff, you did not apply
  it.
- Commit the lever when the work outlives the session.

**Balance:** the bar is triviality, not repetition. A one-off still earns a
lever when the lever is what makes the work checkable. Per
[[laziness-protocol]], build the smallest script that does or proves the job,
never a framework.

Distinct from [[encode-lessons-in-structure]], which makes a recurring
instruction a durable guardrail. This is throughput and reviewability on the
work in front of you. For scripting the verification itself, see
[[prove-it-works]].
