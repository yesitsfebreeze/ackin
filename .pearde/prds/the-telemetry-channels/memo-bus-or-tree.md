---
prd: the-telemetry-channels
claimed: quill 2026-09-12
---

## Bus or tree

The contract left one fork to settle in a memo: does a channel route like a
birth-pipe tree (an event flows parent → child, each hop deciding who sees it)
or is there one host-resident bus every channel lives on?

**Decision: one host-resident bus.** The stream is a single `stream::Stream`
owned by the `Runtime`; every channel is a named log inside it.

Why the bus wins here:

- The profile's cartridges already converge in one daemon. There is exactly one
  `Runtime` per process; a second routing structure beside it would be a second
  source of order to keep deterministic.
- The registries this host already keeps are flat and host-resident: realm
  stores, `on` listeners, the outbox. Channels are the same shape — a name
  mapped to members — so they belong in the same family, not a new topology.
- The socket-only memo's "no broker" rules out a *network* broker and discovery,
  not an in-host registry. The bus is not a broker; it is the daemon's own
  memory, reachable only through the same Unix socket and wire the host already
  serves.

What the tree would have bought, and why it is not owed: per-subscriber routing
decisions made at each hop. But channels are addressed by name, not by
ancestry — a watcher that wants `build` says `build`, and nothing in the PRD
asks for events to flow only along composition edges. Where edge-scoped routing
is wanted, it already exists: listeners (`ctx:on`) are exactly the tree-shaped
story.

Scope boundaries the bus implies:

- A nested sub-host keeps its own stream. Relaying between hosts is transport,
  which is the-wire's concern, not the channels'.
- Retention (how long a log lives) and persistence across daemon restarts are
  out of scope per the contract; the in-memory log is replay-only for the
  daemon's lifetime.