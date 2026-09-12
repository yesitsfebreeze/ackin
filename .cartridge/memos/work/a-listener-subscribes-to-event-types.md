---
kind: work
description: "A cartridge subscribes to the run stream by event type and answers by posting, never by acting on the run directly"
status: open
level: 11
estimate: 1d
needs:
  - "[[an-event-declares-its-type]]"
---

# a-listener-subscribes-to-event-types

## Outcome

A cartridge can listen to what agents are doing without reading a transcript. Each
event a run writes is published with its type, session and run, and a cartridge
registers the types it wants and receives them in order as they are written. That
is how tools connect to each other: a memory, a board bridge, a supervisor or a
metric sees the stream it cares about and nothing else.

A listener's only way back in is a post. It cannot cut a step, answer an approval
or change a phase, because the run task is the one thing deciding what the run
does; a listener that wants the run to change course posts an event and lets
[[a-priority-event-unblocks-the-step]] decide.

A listener never holds a run up. It is dispatched without the run waiting on it,
bounded, and a listener that errors or hangs is dropped and recorded rather than
stalling the agent.

## Check

- [ ] With a listening cartridge in the profile, every event of a run reaches it
      once, in stream order, carrying its type, session and run.
- [ ] A listener registered for one type receives only that type.
- [ ] A listener that errors, and a listener that never returns, leave the run's
      timing and journal unchanged; both are recorded as failed delivery.
- [ ] A listener that posts back produces exactly one event on the stream,
      attributed to that listener and not to the agent.
- [ ] With the listening cartridge removed from the profile, the run's journal,
      projections and telemetry are unchanged, and no subscription remains.
- [ ] `just check` and `just test` pass.

## Approach

Observed 2026-09-12. The host already carries listeners: a cartridge registers one
over the wire and the host dispatches to it (`core/cartridge.rs:407`,
`core/sdk.rs:137,279`, `core/lua.rs:195`). The run already publishes every stage
through `emit`, which stamps `session`, `run`, `seq`, `revision` and the kind
(`builtin/agent/lib.rs:561`). What is missing is that these are two unconnected
facts: `emit` notifies, nobody subscribes by type, and the events a listener would
want are the ones `emit` currently uses for the UI's own liveness.

The delivery guarantee to aim for is the honest one, not the expensive one: this
is a notification channel over a journal that is already durable, so a listener
that missed an event reads the stream, and the subscription does not become a
second source of truth.
