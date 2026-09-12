---
kind: principle
description: Apply when writing stateful logic, or when code branches a lot or repeats a shape assumption across files. Encode the domain in a structure instead of scattered conditionals.
group: architecture
---

# Model the domain

Encode the real domain in a data structure instead of scattering it across
conditionals.

**Why:** Scattered booleans, repeated shape assumptions and branching spread
across files are accidental complexity. A structure that matches the domain
makes invalid states unrepresentable and deletes branches. Choosing it at
write time is cheap; recovering it later reads as a refactor and gets
deferred.

**Reach for structures like these:**

- A state machine instead of scattered booleans, phases or lifecycle checks.
- A typed model instead of loose parameters or repeated shape assumptions.
- A map, registry, lookup table or sum type instead of branching spread across
  files.
- A reducer or command/event model instead of ad hoc state mutations.
- A module organized around one body of domain knowledge instead of a sequence
  such as load, validate, transform, save. Execution order is not ownership.
- A small module boundary that gathers repeated behaviour, ownership or
  invariants.
- A queue, cache, index, tree or normalized collection where the access
  pattern calls for it.
- Any other structure that fits. When none does, work out what the code must
  never allow and how the data gets read, then find the structure that encodes
  exactly that.

Do not force an abstraction. Prefer boring code when the current shape is
already clear, local and unlikely to grow. Be skeptical of an abstraction that
adds indirection without removing branches, duplicated rules, invalid states
or lifecycle risk.

**The tells that you skipped this:** a new feature that grows an existing
if/else chain by one more branch; a second boolean that must stay in sync with
the first; phase-named modules repeating the same domain rules across steps.

The type-level half of this is [[type-system-discipline]].
