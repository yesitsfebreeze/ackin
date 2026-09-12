---
kind: principle
description: Apply when you catch yourself writing the same instruction a second time, or notice a recurring correction. Encode the rule as a check, a type, a lint, or a script instead of more text.
group: meta
---

# Encode lessons in structure

Encode recurring fixes in mechanisms — tests, types, metadata, automation —
instead of textual instructions. Every error, human correction and unexpected
outcome is a learning signal. Capture it, route it, close the loop.

**Why:** Textual instructions are easy to miss. They require the reader to
notice, remember and comply. A structural mechanism enforces the rule without
cooperation.

**The pattern.** When you catch yourself writing the same instruction a second
time:

1. Ask whether this can be a type, a test, a lint rule, a runtime check or a
   script.
2. If yes, encode it and delete the instruction.
3. If no, because it needs judgment, make the instruction more prominent and
   add an example of the failure mode. That is when it becomes a principle
   memo.

**Pick the strongest mechanism.** When more than one would work, choose the
strongest the situation allows — a state that cannot compile, then a check
that fails the gate, then a canonical helper, then a runtime check. Agents and
people copy whatever the surrounding code already does, so a weaker guard
becomes the next template.

**Corollary:** if the fix is structural, use only the structural fix. The
instruction is the symptom.

**The feedback loop:**

- **Capture every correction.** When the human intervenes or a check fails,
  decide whether it is a one-off or a pattern.
- **Route it to the right layer.** One-off becomes a [[note]]; a recurring fix
  becomes a [[routine]] or a gate; a systemic issue becomes a principle.
- **Close the loop.** Do not just record. Apply it now or create the work.

**Anti-patterns:** acknowledging without recording; recording without routing;
fixing one instance while leaving the pattern intact.

[[build-the-lever]] is the throughput sibling — it builds the tool for the
work in front of you; this builds the guardrail that outlives it.
