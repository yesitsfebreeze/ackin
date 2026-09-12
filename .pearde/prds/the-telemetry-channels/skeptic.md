---
prd: the-telemetry-channels
checked: 2026-09-12
commits: cdbf2aa (docs/memo/specs/report), 268f87d + fff7356 + 0893bf3 (the code; 663916d is the lane twin of 0893bf3), 4413ee6 (record)
---

# Skeptic — the-telemetry-channels

## What holds

- **The landed code is where the brief said, roughly.** `cdbf2aa` carries only
  `.pearde/` files; the src changes are `268f87d` (all eleven footprint files),
  `fff7356` (two clippy fixes in `sdk.rs`) and `0893bf3` (the added
  repeat-subscribe test). `663916d` is the lane twin of `0893bf3` (same stat,
  same timestamp). Nothing landed outside the footprint union.
- **The determinism rule holds in `src/stream.rs`.** One `parking_lot::Mutex`
  over `State` (`stream.rs:61`); `publish` (`stream.rs:77`), `subscribe`
  (`stream.rs:85`) and `unsubscribe` (`stream.rs:119`) all take it. The resume
  replay, the registration and the join envelope land in one critical section
  (`stream.rs:89-114`), and `unsubscribe` removes the member and appends the
  leave envelope in one (`stream.rs:119-129`), so a subscribe/unsubscribe
  racing a publish can neither reorder nor lose an envelope. The fan-out
  happens inside the same lock hold (`append`, `stream.rs:152-173`), so no
  subscriber ever sees an envelope ahead of the membership change it follows.
- **spec01: all six boxes have real tests** in `src/tests/stream.rs`. The
  crash-resume test (`a_subscriber_that_crashed_and_came_back…`, seen
  `[2,3,4,5,6,7]` — replay past seq 1, then the live feed, one sequence, no gap
  no duplicate) is the contract's replay/determinism box, and it proves the
  live-queue ordering the in-order box asserts only through the log.
- **The two verify-path corrections are sound.** `cargo test --lib
  tests::stream::a_failing_listener` matches on main (1 passed); the old
  `tests::stream::tests::…` spelling matched nothing. The added
  `a_repeat_subscribe_spawns_no_second_pump` is in the footprint file and
  passes.
- **spec04 box 4 reproduces on main.** I ran the probe's steps against the
  main checkout's binary (the script hardcodes the lane root, so I ran its body
  with `BIN=target/debug/zirkle`): the follow client saw exactly the report's
  three envelopes — join seq 1, CLI publish seq 2, Lua cartridge publish seq 3.
- **The bus-or-tree fork is settled in a committed memo**
  (`memo-bus-or-tree.md`, one host-resident bus, reasoning given).
- **The clippy red I found is exactly what the report disclosed**: `cargo
  clippy --all-targets` on main fails at `src/tests/wire.rs:163`
  (`unnecessary_to_owned`) — the-wire's lint, outside this footprint, honestly
  reported rather than hidden.

## What does not hold

`cargo test --quiet` on main does **not** reliably pass. Three runs: 107
passed/1 failed; 9 passed/2 failed (filtered); 7 passed/1 failed. Across four
parallel runs, `a_failing_listener_publishes_an_error_event_on_its_channel`
failed three times and `a_lua_cartridge_publishes_and_a_late_watcher_replays_
the_log` once — the same report run that claimed green passes is real
(`finished in 0.40s`), but the landed gate is flaky, not green.

Root cause, concrete: `Host::reconcile` (`src/loader.rs:1120`) spawns the
cartridge fiber and returns without awaiting `apply`. Both failing tests
subscribe and then `emit` immediately (`src/tests/stream.rs:228` and
`:257`); when the emit wins the race, `ctx:on` is not registered yet, no
listener runs, no envelope is published, and the test dies at the 5s timeout
(`src/tests/stream.rs:136`). The sibling test
`a_lua_cartridge_watches…` avoids exactly this by waiting for the watcher's
join echo (`src/tests/stream.rs:288`) before publishing, and the wire test
awaits `host.fiber_of("child").unwrap().settled()` (`:329`). The two failing
tests have no such handshake — spec02 box 1 and spec04 box 1 are therefore
proven by a test that fails roughly a quarter of the time under load.

Two boxes are ticked without a check that runs their path:

- **spec02 box 3** (disposing/exiting a watcher announces the leave) claims it
  is "exercised in the same wire test via the process fixture's unsubscribe
  path" — not true of the landed test: the leave envelope it reads is from the
  socket client's explicit `{"unsubscribe"}` (`src/tests/stream.rs:383`); the
  rpc fixture never unsubscribes, and no test disposes or exits a watching
  cartridge and asserts the leave. `link.leave_subs` (`src/cartridge.rs:418`,
  `:430`) is code-read only.
- **spec03 box 2** names the `link.sub_id` early-return in
  `src/cartridge.rs:487`, but the added test drives the socket connection's
  guard (`src/socket.rs:150`) instead — no test repeats `host.subscribe` on a
  process link, so the box as written is still unproven. (The test itself is
  sound for the path it does drive: one join in the log, one copy, zero
  duplicates.)

spec02 box 4 (socket client vanishing mid-feed → announced leave via the weak
sender) is covered only by reference, as the spec itself states; I accept that
as written, but it is adjacency, not a run check.

## Corrections

- `src/tests/stream.rs` — make `a_failing_listener…` and
  `a_lua_cartridge_publishes_and_a_late_watcher_replays_the_log`
  deterministic: await the cartridge's apply completion before the emit (e.g.
  `host.fiber_of("failing").unwrap().settled().await` as the wire test does at
  `:329`), or give them the same outbox handshake `a_lua_cartridge_watches…`
  uses at `:288`. The gate `cargo test --quiet` must pass on repeated runs.
- `.pearde/prds/the-telemetry-channels/specs/spec02.md` — box 3: either add a
  test that disposes/exits the watching child and asserts the leave envelope on
  its channel, or correct the box to say the path is code-read, not exercised.
- `.pearde/prds/the-telemetry-channels/specs/spec03.md` — box 2: either extend
  the repeat test to repeat `host.subscribe` through the rpc fixture and assert
  one daemon-side pump on the link, or correct the box to name the socket
  connection guard (`src/socket.rs:150`) it actually tests.

CHANGE — the landed implementation and its determinism core hold and the probe reproduces on main, but the landed test suite is flaky (two stream tests race the Lua apply and fail up to a quarter of runs, `cargo test` included), and spec02 box 3 / spec03 box 2 are ticked without a check that runs the path they name; fix the two tests and correct or close those two boxes.