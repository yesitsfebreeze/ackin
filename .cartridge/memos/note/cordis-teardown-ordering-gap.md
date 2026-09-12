---
kind: note
description: "The paper's Algorithm 5 drains dependents before any provider inverse runs; the pinned code drains them only inside the provide disposer while the provider's other disposers run concurrently"
date: "2026-09-09"
uses:
  - usage: "[[read-usage]]"
    when: ["relying on Cordis to keep a provider's resources alive until dependents finish cleanup"]
    tags: [research]
---

# cordis-teardown-ordering-gap

Paper §5.1.3, Algorithm 5 line 25: `unload` first marks the fiber UNLOADING,
notifies dependents, awaits each of them reaching INACTIVE, and only then runs
`fiber.dispose`. The text says the wait sits ahead of the whole recovery
because the disposers run concurrently and a wait inside one would leave the
rest unordered. Theorem 70 rests on this.

Pinned source ([[cordis-repo]]) does something narrower. `_unload` starts
every top-level disposer at once with `Promise.all`
(https://github.com/cordiverse/cordis/blob/f8ea3cd50f1a5724e8e715995bcde131c9c12b2c/packages/core/src/fiber.ts#L399-L459); the wait for dependents lives in
the disposer that `provide()` installed, which removes the binding and then
`Promise.allSettled`s the affected fibers
(https://github.com/cordiverse/cordis/blob/f8ea3cd50f1a5724e8e715995bcde131c9c12b2c/packages/core/src/reflect.ts#L177-L204). That delays only the
registration's own disposer. A socket or database closer the provider
registered as a separate effect can run while a dependent's async cleanup
still uses it.

glue takes the paper's order: `unload` in `src/fiber.rs` awaits every dependent before it runs one inverse, and `src/tests/lifecycle.rs` holds it there. The upstream finding is read by control flow, not reproduced by a runtime test; a failing test or an
upstream change to `_unload` would settle it. Context:
[[cordis-revertible-effects]], [[cordis-guarantees-and-limits]].
