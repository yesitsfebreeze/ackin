---
complexity: 6
footprint:
  - src/sdk.rs
  - src/tests/fixtures/rpc_fixture.rs
---

# spec03 — the SDK surface: publish and subscribe for process cartridges

A process cartridge speaks channels through `sdk::Host` without knowing the
wire: `host.publish(channel, data)` writes `{"publish": ch, "data"}` down the
link and is one write — a publish with no subscriber costs the daemon one
append, no round trip. `host.subscribe(channel, f)` writes
`{"subscribe": ch}` and spawns a pump task that feeds an ordered queue and
calls the boxed handler in arrival order; `host.unsubscribe(channel)` removes
the handler and writes `{"unsubscribe": ch}`. The run loop routes daemon
frames that name a channel to the stored watcher's queue, so delivery never
interleaves with call replies. A cartridge that dies mid-watch is covered by
the link-level teardown in spec02, not by the SDK.

The rpc fixture grows a `publish` passthrough (its `roundtrip` handler sees a
`publish` arg and publishes instead of echoing) and a `telemetry` config that
subscribes to `build` and re-sends what it receives as an outbox event — the
composition tests' echo witness.

## Acceptance

- [x] a process cartridge that subscribes receives what a socket client
      publishes, in order, each envelope carrying `kind/seq/from/data` —
      closed by spec02's wire test
      (`tests::stream::a_process_cartridge_publishes_and_watches_over_the_wire`),
      whose echo seqs are exactly the three publishes' sequences
- [x] `subscribe` is idempotent on the link: a second `host.subscribe` for the
      same channel does not create a second daemon-side pump (guarded by
      `link.sub_id` early-return in `cartridge::handle`) — no test exercised the
      repeat, so this worker added
      `tests::stream::a_repeat_subscribe_spawns_no_second_pump` (one join in the
      log, one copy of the publish, zero duplicates)
- [x] a watcher echoes what arrives even when call replies interleave with
      channel frames — the wire test collects its echo seqs and the socket
      event in either order and still finds one gapless sequence

## Verify and Proof

```sh
cd /Users/feb/dev/cartridge/.pearde/.lanes/the-telemetry-channels
cargo test --lib tests::stream::a_process_cartridge
```

The run that wrote this spec: `cargo test --offline --lib` → 85 passed, 0
failed.