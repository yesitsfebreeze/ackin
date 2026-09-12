---
kind: work
description: "A cartridge spawned inside another gets an id, a memory bank, a landscape row and its own replacement, like any profile entry"
status: active
owner: "sys-38/implementer-a-nested-cartridge-is-a-first-class-entry"
level: 10
estimate: 2d
---

# a-nested-cartridge-is-a-first-class-entry

## Outcome

A cartridge composed inside another cartridge is an entry in every way the host
already means it: it has a stable id qualified by its parent, its own memory
bank and versioned snapshot, a landscape row naming the parent that composed it,
and it is replaced in place when its own sources change — without restarting the
parent that holds it. A nested subtree is therefore inspectable and reloadable
by the same operations as a top-level entry.

Scope is the lifecycle of children spawned through the existing nesting
primitive. Reusing that ownership for the host profile itself is
[[the-profile-is-the-root-composer]].

## Check

- [ ] A fixture parent composing one child shows both in `tool.memo` landscape
      rows, the child naming its parent and its own generation.
- [ ] Editing only the child's source replaces the child: its fiber uid changes,
      the parent's uid does not, and the parent's service keeps answering across
      the switch.
- [ ] The child's bank survives its own replacement: a value checkpointed before
      the edit is readable by the replacement, and a stale revision write fails.
- [ ] A child that fails to apply leaves the parent active and reports one line
      naming the child, and the parent's other children keep serving.
- [ ] Disposing the parent disposes the child first; a test asserts the order
      from the children's own disposal events.
- [ ] `just check` and `just test` pass.

## Approach

Observed 2026-09-12. `ctx:cartridge(path, config)` already spawns a child
cartridge as a fiber inside the caller's context (`core/context.rs:138`) and
returns a handle with `uid`, `state`, `error`, `dispose` and `wait`. Nothing in
`builtin/` calls it: the primitive is present and unused. What a child does not
get is everything `Loader` keys by profile entry id — bank (`core/loader.rs:677`),
landscape row, generation, and per-entry replacement. Its sources are only
collected so the foreground build rebuilds them (`core/loader.rs:966`), so today
a change under a child rebuilds and reloads through the parent.

## Spec

Give the child an entry identity: `parent-id/child-id`, with `child-id` from the
`ctx:cartridge` call and stable across replacement. Hold children in the same
structure that holds top-level entries, with a parent link, so the bank, the
snapshot, the source stamps and the generation come from one code path instead of
two. `nested_sources` stops being a separate map; a child's sources are the
child's, like any entry's.

Replacement of a child is the existing transaction (prepare, drain, freeze bank,
hand the candidate a private copy, publish only on matching provided keys) run
against the child fiber. The parent is a consumer during that switch, exactly as
a top-level consumer is when its provider is replaced; nothing in the parent's
code has to know a replacement happened. A child whose provided keys change
still requires recomposition, unchanged from today's rule.

Landscape rows gain a parent column, so the graph shows the tree rather than a
flat list — [[the-landscape-owns-the-graph]] keeps owning the projection, this
only adds the edge it is missing.

Disposal is depth-first: children before the parent, which is what the runtime's
dependency order already gives once children are entries rather than side effects.
