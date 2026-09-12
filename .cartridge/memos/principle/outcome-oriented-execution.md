---
kind: principle
description: Apply during planned rewrites and migrations with explicit phase boundaries. Converge on the target architecture; do not preserve smooth intermediate states with throwaway compatibility code.
group: core
---

# Outcome-oriented execution

Optimize for the intended, verifiable end state rather than preserving smooth
intermediate states.

**Why:** Keeping every intermediate step fully stable creates temporary
compatibility code that becomes long-lived debt. Converge on the target
architecture and prove correctness at explicit verification boundaries.

**The rule:**

- Prioritize end-state integrity over transitional stability.
- Intermediate breakage is acceptable when it is planned, scoped and
  reversible.
- Always run final verification before declaring done, per [[prove-it-works]].

**Guardrails:**

- Use this for planned rewrites and migrations with explicit phase boundaries,
  not for ordinary change.
- Declare up front where temporary breakage is acceptable.
- Keep high-signal checks on actively touched areas while migrating.
- Require full static and runtime verification at plan completion.

The API-shaped case of this is
[[migrate-callers-then-delete-legacy-apis]].
