---
kind: principle
description: Apply when sequencing an addition, refactor, or rewrite. Remove dead code, redundant validators, and stub references first, then build on the simpler base.
group: core
---

# Subtract before you add

When evolving a system, remove complexity first, then build.

**Why:** Adding to a complex system compounds complexity. Removing first
leaves less code, reveals the essential structure, and usually makes the next
design obvious. Default to subtraction.

Make simplification a continual investment. Leave the design slightly simpler
and more capable behind the same or smaller surface than you found it.

**The pattern:**

- Sequence removal before construction.
- Cut before you polish. Reach the minimum before investing in quality.
- Design for observed usage, not speculative edge cases.
- No speculative validators, parsers or guards beyond what the spec demands.
- Simplify prose too: redundant instructions, excessive templates.
- A reference with no novel content gets deleted, not left as a stub.

[[laziness-protocol]] is the same bias applied to the diff in front of you.
[[foundational-thinking]] sequences scaffold before features; subtraction comes
before scaffolding.
