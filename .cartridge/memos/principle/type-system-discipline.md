---
kind: principle
description: Apply when designing types or reviewing a signature in any statically typed language. Make illegal states unrepresentable, brand semantic primitives, parse external data at boundaries, exhaust variants, derive from authoritative schemas.
group: architecture
---

# Type system discipline

The type checker is a proof assistant. Use it to eliminate impossible states,
mismatched primitives and unhandled variants at compile time. A case the types
let you ignore becomes a runtime failure the compiler could have stopped.
Prefer defining errors and special cases out of existence over proliferating
handlers.

**The patterns:**

- **Make illegal states unrepresentable.** Model variants as sum types: enums
  with payloads in Rust, discriminated unions in TypeScript, sealed classes in
  Kotlin. Do not model state as a bag of optional fields where contradictory
  combinations compile. `{ completed: bool, completed_at: Option<Date> }`
  admits `completed` with no date, which is meaningless. Derive the boolean
  from one source, or model the variants explicitly. If a bug forces the
  question "wait, can this combination actually happen?", the type is too
  loose.
- **Types are constructions, not restrictions.** Build the type up from the
  values you want instead of carving them out of a looser type with checks. A
  non-empty list is a head plus a rest, not a list with a length check. A valid
  time range is a start plus a duration, not two timestamps you must keep
  ordered. Choose the shape that cannot build the illegal value and expose the
  interface callers need on top.
- **Brand semantic primitives.** `UserId` and `OrderId` are strings underneath
  and must not be interchangeable. Newtypes in Rust, opaque types in Swift,
  branded intersections in TypeScript. Validate once at creation, trust the
  type downstream.
- **External data is untyped until parsed.** RPC payloads, JSON, IPC messages,
  CLI args, config files, environment variables, database rows. One parse
  function at every boundary turns unstructured input into the typed model.
  See [[boundary-discipline]] for where the validation goes.
- **Do not lie to the type system.** Casts, unsafe coercions and assertions
  that bypass the compiler are latent runtime crashes. If the compiler cannot
  prove a fact, prove it — validate, narrow, refine the model — or accept that
  the cast is a hazard.
- **Exhaustive matching is the compiler's job.** Matching on a sum type must
  fail compilation when a new variant appears unhandled. Use the idiom the
  language provides.
- **Derive types from authoritative schemas.** When a protocol definition,
  OpenAPI spec, migration or token file defines a shape, derive from it
  instead of hand-rolling a parallel type. See
  [[encode-lessons-in-structure]].
- **Strengthen a type only where partiality appears.** A runtime assertion,
  null check or "this should never happen" marks the place a type is too weak.
  Push that check up into the type, then stop. The job is to track the cases
  each use site must handle, not to describe the data as precisely as
  possible. `sum` of an empty list is zero, so it takes the plain list; `head`
  of an empty list has no answer, so it demands the non-empty one.

**The tests:**

- Can I write a comment explaining when this combination of fields is valid?
  Then the type is too loose. Split it into a sum type.
- Do two arguments share a primitive type but mean different things? Brand
  them.
- Where did this cast or unchecked unwrap come from? Trace it to the boundary
  and validate there.
- If a new variant is added next month, will the compiler say where to add a
  case?
- Am I strengthening this type to keep an operation total, or just to be more
  precise? If nothing would otherwise panic, keep the plain type.

The value-level half of this is [[model-the-domain]].
