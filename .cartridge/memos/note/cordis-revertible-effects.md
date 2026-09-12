---
kind: note
description: "In Cordis every context mutation is one ctx.effect whose callback yields inverses; the runtime composes and runs them, but never checks that an inverse actually reverts"
date: "2026-09-09"
uses:
  - usage: "[[read-usage]]"
    when: ["asking how Cordis undoes a component's side effects, and what it cannot undo"]
    tags: [research]
---

# cordis-revertible-effects

The paper's temporal composability rests on one primitive: `ctx.effect(cb)`
(§5.1.1, Algorithm 1). The callback is an effect function or an iterator;
each step yields an inverse, the engine prepends it to a composite so recovery
runs last-in-first-out, and a guard consulted before each step lets an
in-flight activation stop at an iteration boundary holding only the inverses
it has. Coeffect provision (`set`), component instantiation (`use`), and
event listeners all reduce to it, so unloading a parent cascades to children.

The paper states the limit itself: the runtime does not verify that an
inverse reverts its effect — that is an obligation on the author (§5.1.1), and
anything outside the system boundary (bytes written, datagrams sent) is
`id` on the context, never reverted (§6.1). Compensation is named as the
application's own coarser equivalence.

Source, pinned in [[cordis-repo]]: `ctx.effect` accepts a disposer, a
promise of one, or a sync/async iterable of them
(https://github.com/cordiverse/cordis/blob/f8ea3cd50f1a5724e8e715995bcde131c9c12b2c/packages/core/src/fiber.ts#L229-L339). Within one effect the collected
disposers run in reverse; across a fiber's top-level effects disposal is
started in reverse order but awaited with `Promise.all`, so there is no
sequential LIFO across effects (https://github.com/cordiverse/cordis/blob/f8ea3cd50f1a5724e8e715995bcde131c9c12b2c/packages/core/src/fiber.ts#L439-L459).
Cancellation is cooperative: a never-settling step stalls teardown.

What would change this: an upstream test showing ordered cross-effect
teardown, or a runtime check on inverses. Sibling findings:
[[cordis-teardown-ordering-gap]], [[cordis-reactive-coeffects]].
