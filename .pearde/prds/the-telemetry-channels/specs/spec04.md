---
complexity: 8
footprint:
  - src/context.rs
  - src/socket.rs
  - src/main.rs
---

# spec04 — every audience: Lua `ctx:publish`/`ctx:subscribe`, socket subscribe/resume, CLI

Three surfaces over the same bus:

- **Lua**: `ctx:publish(channel, payload)` publishes attributed to the
  cartridge; `ctx:subscribe(channel, f)` joins with a Lua handler whose pump
  converts each envelope to a Lua value and calls the handler in order; a Lua
  error inside the handler unsubscribes, and the subscription is a tracked
  effect (`effect_sync`) so disposing the fiber unsubscribes.
- **Socket**: a request may name `subscribe` (with `since` to resume from the
  last sequence the watcher saw — replay past it, then the live feed, one
  sequence), `unsubscribe`, or `publish`; delivery arrives as
  `{"channel": ch, "event": envelope}` on the same connection as every other
  frame. Subscriptions are idempotent per connection, and the pump detects
  disconnect through a weak sender and announces the leave.
- **CLI**: `zirkle publish <ch> '<json>'` and `zirkle follow <ch>` — follow
  prints the channel frames (and error frames) as they arrive, so a person
  watches a channel without writing a client. (`zirkle tail` is the outbox
  reader and stays so; it does not see channel frames.)

## Acceptance

- [x] a Lua cartridge publishes with one line and a late socket watcher that
      subscribes with `since: 0` replays the log from the start, its own join
      arriving after the replay, on one sequence —
      `tests::stream::a_lua_cartridge_publishes_and_a_late_watcher_replays_the_log`
- [x] a Lua cartridge subscribes and receives what a socket client publishes,
      in order, join first —
      `tests::stream::a_lua_cartridge_watches_a_channel_a_socket_client_publishes_to`
- [x] a resuming subscriber's replay is bounded exactly at its resume point and
      is ordered with the live feed —
      `tests::stream::a_subscriber_that_crashed_and_came_back_ends_up_where_it_was`
- [x] the probe drives the real binary end to end: a daemon over a throwaway
      profile, a Lua cartridge publishing on an event, a `follow` client that
      joins and then sees the CLI's publish and the cartridge's publish as
      ordered envelopes —
      `sh /Users/feb/dev/cartridge/.pearde/prds/the-telemetry-channels/probe/end-to-end.sh`
      from the lane (its follow output is the report's evidence)

## Verify and Proof

```sh
cd /Users/feb/dev/cartridge/.pearde/.lanes/the-telemetry-channels
cargo test --lib tests::stream::
```

The run that wrote this spec: `cargo test --offline --lib` → 85 passed, 0
failed; the probe built the real binary and captured follow output.