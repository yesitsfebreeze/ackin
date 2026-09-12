---
kind: note
description: "A Cordis fiber activates when every declared key resolves to an ACTIVE provider and reloads when the providing fiber changes; an in-place value overwrite is not observed"
date: "2026-09-09"
uses:
  - usage: "[[read-usage]]"
    when: ["asking how Cordis dependency injection drives plugin lifecycle"]
    tags: [research]
---

# cordis-reactive-coeffects

Spatial composability in the paper (§3.2, §5.1.2–5.1.3): a component declares
`inject` (its coeffect specification); `set(k, v)` binds a key in a store
under a realm symbol and notifies every live fiber that injects that key in
the same realm; `refresh` recomputes the fiber's target as the tuple of
provider fiber uids, and a changed target starts a reload or unload. Because
the target is provider identity, not value, a provider that overwrites its own
binding in place is not seen; to propagate it must withdraw and re-provide
(§5.1.3). A dependency cycle simply leaves both fibers inactive forever,
predictable from declarations alone (§6.5). Transitions are inertial: one runs
to completion before the next is judged (§4.4).

Source ([[cordis-repo]]): current names are `ctx.plugin()` to instantiate
and `ctx.provide(name, value)` to bind, not the paper's `use`/`set`; a
duplicate provider for one name is refused. The epoch is built from provider
fiber ids and a missing dependency yields an inactive epoch
(https://github.com/cordiverse/cordis/blob/f8ea3cd50f1a5724e8e715995bcde131c9c12b2c/packages/core/src/fiber.ts#L371-L459). `ctx.set()` today only mutates
an existing binding owned by the calling fiber and notifies nobody
(https://github.com/cordiverse/cordis/blob/f8ea3cd50f1a5724e8e715995bcde131c9c12b2c/packages/core/src/reflect.ts#L164-L228). `await ctx.plugin()` waits for
current lifecycle work, not for dependencies, so a plugin can stay PENDING.
A failed fiber does not retry when its dependency returns; `update()` clears
the failure.

What would change this: the rc line renaming its API again, or the paper's
next revision adopting the code's names. See [[cordis-revertible-effects]].
