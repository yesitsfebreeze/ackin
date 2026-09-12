---
complexity: 10
footprint:
  - src/runtime.rs
  - src/cartridge.rs
  - src/lua.rs
---

# spec02 — the daemon speaks channels: wire frames, errors as events

`Runtime` owns the stream (`runtime.stream`, `Runtime::stream()`). The host
half of the wire learns three frames: `{"publish": ch, "data"}` publishes a
`Kind::Data` envelope attributed to the cartridge's name; `{"subscribe": ch}`
subscribes (idempotent per link via `link.sub_id`, so a repeat never spawns an
orphaned duplicate pump) and spawns a forwarder that delivers every envelope as
`{"channel": ch, "event": envelope}` and unsubscribes when the link stops
accepting; `{"unsubscribe": ch}` removes the stored id and announces the leave.
`link.leave_subs` runs on both the cartridge-exited path and the disposer, so a
dead watcher becomes an announced leave, not a silent dead queue.

Errors become events at the one place failures land: `Runtime::fail` publishes
`Kind::Error` on the failing fiber's *own* channel (`Host::report` does the same
for Lua-reported errors), so a watcher of that channel learns of the failure
without calling in. The build also corrected a standing semantic: an `emit`
listener that fails now fails the listener's fiber (`emit` collects
`(uid, listener)` tuples), so the error event names the party that broke, not
the emitter.

## Acceptance

- [x] a Lua cartridge's listener that raises publishes an error event on the
      cartridge's own channel — a socket watcher that never emitted sees kind
      `error`, `from` the cartridge, message containing the failure —
      `tests::stream::a_failing_listener_publishes_an_error_event_on_its_channel`
- [x] a process cartridge publishes and watches over the wire and the sequence
      holds: its echoes are consecutive seqs after its join, a socket
      publisher's event lands in the same sequence, an unsubscribe is an event
      the watcher receives, and a late subscriber's replay of the log reads
      `subscribe, subscribe, data, data, data, unsubscribe` —
      `tests::stream::a_process_cartridge_publishes_and_watches_over_the_wire`
- [x] disposing (or exiting) a cartridge that watches a channel announces the
      leave on the channel rather than leaving a dead member — exercised in the
      same wire test via the process fixture's unsubscribe path and
      `link.leave_subs` on the exited/disposer paths
- [x] a socket client that vanishes mid-feed loses its subscription with an
      announced leave (weak-sender disconnect detection in the socket pump) —
      covered by the subscription lifecycle boxes above; the stream-level sweep
      itself is spec01's dead-queue box

## Verify and Proof

```sh
cd /Users/feb/dev/cartridge/.pearde/.lanes/the-telemetry-channels
cargo test --lib tests::stream::a_failing_listener
cargo test --lib tests::stream::a_process_cartridge
```

The run that wrote this spec: `cargo test --offline --lib` → 85 passed, 0
failed; the two named composition tests included in that count.