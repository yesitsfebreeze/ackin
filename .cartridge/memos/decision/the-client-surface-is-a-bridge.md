---
kind: decision
description: "Core's client surface is a neutral bridge (status plus scoped calls); any client attaches, terminal ui is one of them"
status: accepted
date: "2026-09-12"
uses:
  - usage: "[[read-usage]]"
    when: ["touching the loader catalog, ui_call dispatch, or adding a second client"]
---

# the-client-surface-is-a-bridge

## Decision

The block in `core/loader.rs` and `core/cartridge.rs` that today calls itself
`ui_catalog`/`ui_call`/`is_ui_host` stays in core but is renamed as a client-neutral
surface: a `bridge` module exposing `bridge.status` (active entries, generations,
scopes) and `bridge.call` (owner- and generation-scoped service calls). The
`ui_host` flag becomes a bridge-enabled flag. `builtin/ui` is the one shipped
client; a desktop client later attaches through the same surface with no core
change.

## Why

The flow was never UI-specific: a client asks what is live, then makes scoped
calls over the wire. Naming it after the one current client is a concern
inversion — the concept belongs to the host, the rendering belongs to the
client. The loading half of `loader.rs` (reconcile, replace, watch) keeps its
name: that part genuinely loads.

## Consequences

Wire key `ui_call` becomes `bridge` naming, sdk conveniences rename
accordingly, and `core/tests/ui.rs` moves with them. `development.rs` also
stays in core: it is the reload pipeline's build step with exactly one
consumer, and a pluggable build hook for one implementation is not worth its
interface. See [[the-landscape-owns-the-graph]] for the piece that does
leave core.
