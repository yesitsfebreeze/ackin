---
complexity: 12
footprint:
  - src/stream.rs
  - src/lib.rs
  - src/tests/stream.rs
---

# spec01 — the stream: named channels, an ordered replayable log, announced membership

`stream::Stream` is one host-resident bus (see `../memo-bus-or-tree.md`): a
mutex over `next` and a channel map, each channel holding a per-channel
sequence, a retained in-memory log and its subscribers. `publish` appends
`{"ch","seq","from","kind","data"}` and fans out; with no subscriber it costs
one append and the payload is not copied to anyone. `subscribe(owner, after)`
replays the log past `after` (`partition_point` over the seq-ordered log, so
replay and live feed are one sequence) into the new subscription's queue, then
appends the subscriber's own join envelope under the same lock — so a second
replay of the same log yields the same state, and the join is ordered with
everything else. `unsubscribe` removes the member and appends the leave
envelope; a leave for an unknown id is a no-op. Publishing sweeps subscribers
whose receiver is gone, so a dead queue never holds the log. Errors are a
`Kind::Error` envelope like any other.

This unit stands in the lane (built, not committed — `prds/the-telemetry-channels/probe/`
holds the probe); the boxes are what an implementer re-verifies and a later
worker must not break. Retention (bounded logs, eviction) is out of scope per
the contract; the log lives for the daemon's lifetime.

## Acceptance

- [x] a publish with no subscriber costs exactly one append and the log keeps
      it: seqs `[1, 2]`, envelope fields `ch/seq/from/kind/data` all present,
      and an unrelated channel replays empty —
      `tests::stream::publishing_without_a_listener_costs_one_append_and_keeps_the_log`
- [x] a subscriber receives everything published to its channel in order, its
      own join first, and the join is in the log —
      `tests::stream::a_subscriber_receives_everything_published_to_its_channel_in_order`
      (seqs `[1, 2, 3, 4]`, log[0] kind `subscribe` from the watcher)
- [x] a subscriber that crashed and came back, resuming at the last sequence it
      saw, ends up where it was: replay past the resume point then the live
      feed, one sequence, no gap and no duplicate —
      `tests::stream::a_subscriber_that_crashed_and_came_back_ends_up_where_it_was`
      (seen `[2, 3, 4, 5, 6, 7]`)
- [x] leaving is an event everyone still on the channel sees (kind
      `unsubscribe`, `from` names the leaver), and an unknown-id leave is
      silent — `tests::stream::leaving_is_an_event_everyone_still_on_the_channel_sees`
- [x] a queue nobody reads is swept at the next publish and the sequence keeps
      moving — `tests::stream::a_queue_nobody_reads_ends_at_the_next_publish`
- [x] concurrent publishers with no coordination leave a gapless ordered log,
      every sequence accounted for exactly once —
      `tests::stream::concurrent_publishers_leave_a_gapless_ordered_log`
      (4 × 25 publishes, seqs exactly `1..=100`)

## Verify and Proof

```sh
cd /Users/feb/dev/cartridge/.pearde/.lanes/the-telemetry-channels
cargo test --lib tests::stream::
```

The run that wrote this spec: `cargo test --offline --lib` → 85 passed, 0
failed (baseline 75), three consecutive green runs; `cargo clippy --offline
--all-targets` clean.