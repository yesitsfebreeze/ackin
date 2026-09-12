---
kind: type
type: resolver
description: An evidence-backed assessment of one target usage and revision
---

# Resolver

A resolver memo has description, `target` (wiki link or canonical memo path),
`usage` (reference to a usage memo), `revision` (SHA-256 of the target declaration),
quoted `date`, `verdict`, `status`, and `evidence` (a list of event IDs or check
references). Verdict is helped, missed, misfired, or unknown. Status is proposed,
applied, or dismissed. Unknown may have no evidence; other verdicts need sources.

Write Observation, Evidence, Proposed change, and Result. Identify the situation,
what was observed, and the exact declaration field or routine section to change.
An access is not proof of usefulness. Counts from `resolver` describe only the
retained observation window. `observe` records explicit caller reports; it does
not independently verify the reported action or success.

Read the target's revision before a write and pass `expected_revision`; a conflict
requires rereading. Apply the lesson through that ordinary write, then record its
result here. Applied is the author's assertion, not an automatic two-file transaction.
There is no automatic rating, deletion, or promotion to system instructions.
