---
kind: principle
description: Apply when wiring validation, error handling, or framework adapters. Concentrate guards at system boundaries; trust internal types and keep business logic in pure functions.
group: architecture
---

# Boundary discipline

Place validation, type narrowing and error handling at system boundaries.
Trust internal code unconditionally. Business logic lives in pure functions.
The shell around it is thin and mechanical.

**Why:** Scattered validation is noisy, redundant, and gives a false sense of
safety. Keeping logic out of framework wiring lets it be tested without the
framework.

**The pattern:**

- **At boundaries** — CLI args, config files, external APIs, network protocols
  — validate, return errors, handle defensively.
- **Inside the system** — typed data, error propagation, no re-validation.
  Trust the types.
- **Across the boundary** — expose domain concepts, not the boundary's private
  representation. Keep general-purpose mechanism inside and special-purpose
  policy at the edge.

**Applications.** Validate config at parse time, not inside business logic.
Parse raw data into domain types at the boundary. Do not re-export transport,
storage, framework or wire types through the public surface. Drop redundant
nil checks deep in a call chain the boundary already validated. Keep parse
functions pure transforms from bytes to typed state, and scoring or assessment
pure transforms from state to results.

**The tests:**

- Is this data crossing a system boundary right now? If not, the validation is
  redundant.
- Can this be a pure function the shell just calls? If yes, extract it.

Where the types for that boundary come from is [[type-system-discipline]].
