---
state: done
origin: requested
priority: 72
complexity: 36
blast-radius: mid
needs:
  - the-wire
actual: 0h
commit: 663916d cdbf2aa
---



# The telemetry channels

Every cartridge publishes what it is doing, and any other cartridge can listen.

The requirement as it arrived, verbatim:

> "the base cartridge should have a telemetry system where it can publish and
> broadcast events in channels essentially. That way we can get a good idea from
> other plugins of what is going on, what is not working, errors, all of that. So
> it really lends itself to agentic swarm engineering. The harness that we are
> building here is basically comprised of many cartridges that do a very specific
> job very well. Therefore the communication layer must be replayable,
> deterministic, accurate, fast, and small. Thus I thought of a JSON ledger of
> events that we can just stream into other agents. So one agent can subscribe to
> a channel and then he can listen to all the events in that channel. He can
> unsubscribe from it so everybody knows, in essence, what is going on with the
> other cartridge. And this is the end goal but the base system must make this as
> easy as possible."

Restated as a contract:

- A cartridge publishes an event to a named channel. It does not know who is
  listening, and it costs it nothing to find out that nobody is.
- Another cartridge subscribes to a channel by name and receives every event
  published to it, in order, from the point it subscribed. It unsubscribes, and
  both the subscription and its end are themselves events on the channel — so
  every participant knows who is watching.
- The stream is a JSON event log: one object per line, append-only, ordered.
- **Replayable and deterministic.** The same log replayed twice produces the
  same state twice. A subscriber that crashed and came back is handed the same
  events in the same order and ends up where it was. This is the requirement,
  not a property that would be nice to have.
- **Accurate.** An event that was published appears exactly once. An event that
  was not published does not appear.
- **Fast and small.** Publishing sits on the hot path of every cartridge, so
  the cost with no subscriber must be near zero, and the envelope must not
  dwarf the payload it carries.

Errors are events. A cartridge that fails publishes the failure on its channel
rather than only returning it to whoever called — the caller is not the only
party that needed to know.

This is the base layer, not the consumer. The end use is agentic: an agent
subscribes to a channel and reads another cartridge's work live — what it did,
what broke, what it is waiting on — so a swarm of cartridges is legible from
outside. This PRD builds what makes that trivial: the publish call in the SDK,
the channel registry, the subscribe and unsubscribe protocol on the wire, and
the event envelope. It does not build the agent.

In scope, and to be settled here: whether events travel only along birth pipes
— a node knows its children, so a subscription routes up the tree and back down
— or through one host-resident bus every node reaches. The first keeps the
vision's "no discovery, no broker". The second is simpler and is a broker.
Decide it, and put the reasoning in a memo.

Not in scope: retention policy, on-disk persistence beyond what replay needs,
and any query language over the log. A consumer that wants those reads the
stream and builds them itself.

At the end, a cartridge publishes an event in one line with no setup, a second
cartridge subscribes to that channel by name and receives everything on it in
order, a replay of that log reaches the same state, and unsubscribing is
visible to everyone on the channel.

A word on names: on this board **ledger** already means the registry of every
cartridge installed on this machine — see [[the-ledger]]. The JSON ledger of
events asked for above is called the **stream** here, so the two never collide.
[[the-wire]] is what carries it.

## History

**failed, retried 2026-09-12 20:27**

**2026-09-12 20:27 — the lane will not rebase**

`lane/the-telemetry-channels` does not land on `session/s25699`; 1 file(s) disagree:

- `src/tests/mod.rs`

Nothing is lost: the worker's commits are on `lane/the-telemetry-channels` and the checkout never moved. `pearde retry the-telemetry-channels` puts a worker back on that lane to rebase it onto `session/s25699` and resolve.

**failed, retried 2026-09-12 20:28**

**2026-09-12 20:27 — the lane will not rebase**

`lane/the-telemetry-channels` does not land on `session/s25699`; 1 file(s) disagree:

- `src/tests/mod.rs`

Nothing is lost: the worker's commits are on `lane/the-telemetry-channels` and the checkout never moved. `pearde retry the-telemetry-channels` puts a worker back on that lane to rebase it onto `session/s25699` and resolve.

## Report

spec01: exit 0

running 11 tests
test tests::stream::leaving_is_an_event_everyone_still_on_the_channel_sees ... ok
test tests::stream::a_subscriber_receives_everything_published_to_its_channel_in_order ... ok
test tests::stream::a_queue_nobody_reads_ends_at_the_next_publish ... ok
test tests::stream::publishing_without_a_listener_costs_one_append_and_keeps_the_log ... ok
test tests::stream::concurrent_publishers_leave_a_gapless_ordered_log ... ok
test tests::stream::a_lua_cartridge_publishes_and_a_late_watcher_replays_the_log ... ok
test tests::stream::a_failing_listener_publishes_an_error_event_on_its_channel ... ok
test tests::stream::a_lua_cartridge_watches_a_channel_a_socket_client_publishes_to ... ok
test tests::stream::a_subscriber_that_crashed_and_came_back_ends_up_where_it_was ... ok
test tests::stream::a_repeat_subscribe_spawns_no_second_pump ... ok
test tests::stream::a_process_cartridge_publishes_and_watches_over_the_wire ... ok

test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 97 filtered out; finished in 0.40s

    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.05s
     Running unittests src/lib.rs (target/debug/deps/zirkle-9dda9247c73c08cf)

spec02: exit 0

running 1 test
test tests::stream::a_failing_listener_publishes_an_error_event_on_its_channel ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 107 filtered out; finished in 0.00s


running 1 test
test tests::stream::a_process_cartridge_publishes_and_watches_over_the_wire ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 107 filtered out; finished in 0.31s

    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.02s
     Running unittests src/lib.rs (target/debug/deps/zirkle-9dda9247c73c08cf)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.02s
     Running unittests src/lib.rs (target/debug/deps/zirkle-9dda9247c73c08cf)

spec03: exit 0

running 1 test
test tests::stream::a_process_cartridge_publishes_and_watches_over_the_wire ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 107 filtered out; finished in 0.31s

    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.02s
     Running unittests src/lib.rs (target/debug/deps/zirkle-9dda9247c73c08cf)

spec04: exit 0

running 11 tests
test tests::stream::a_queue_nobody_reads_ends_at_the_next_publish ... ok
test tests::stream::a_subscriber_receives_everything_published_to_its_channel_in_order ... ok
test tests::stream::leaving_is_an_event_everyone_still_on_the_channel_sees ... ok
test tests::stream::publishing_without_a_listener_costs_one_append_and_keeps_the_log ... ok
test tests::stream::concurrent_publishers_leave_a_gapless_ordered_log ... ok
test tests::stream::a_lua_cartridge_publishes_and_a_late_watcher_replays_the_log ... ok
test tests::stream::a_lua_cartridge_watches_a_channel_a_socket_client_publishes_to ... ok
test tests::stream::a_failing_listener_publishes_an_error_event_on_its_channel ... ok
test tests::stream::a_subscriber_that_crashed_and_came_back_ends_up_where_it_was ... ok
test tests::stream::a_repeat_subscribe_spawns_no_second_pump ... ok
test tests::stream::a_process_cartridge_publishes_and_watches_over_the_wire ... ok

test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 97 filtered out; finished in 0.30s

    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.02s
     Running unittests src/lib.rs (target/debug/deps/zirkle-9dda9247c73c08cf)
