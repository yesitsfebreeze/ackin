Verdict: DONE

Built the whole contract in the lane (uncommitted, per the brief) and wrote
four specs over it. The PRD's one fork — channels routed as a birth-pipe tree
or one host-resident bus — settled in `memo-bus-or-tree.md` (this folder; ride
it to `.pearde/memos/` on commit, since the brief's write boundary holds this
worker to `prds/the-telemetry-channels/`): **one bus**, because a profile's
cartridges already converge in one runtime and the realm stores, `on`
listeners and outbox are already flat host-resident registries — a channel map
is the same shape. The socket-only memo's "no broker" bars a network broker,
not an in-host registry.

## Specs (sum 36, count 4 — under both ceilings)

| spec | carries | complexity | footprint |
|---|---|---|---|
| `specs/spec01.md` | the stream: named channels, ordered replayable log, announced membership, lazy sweep | 12 | `src/stream.rs`, `src/lib.rs`, `src/tests/stream.rs` |
| `specs/spec02.md` | daemon speaks channels: wire frames, idempotent link subs, errors as events, emit-failure semantic fix | 10 | `src/runtime.rs`, `src/cartridge.rs`, `src/lua.rs` |
| `specs/spec03.md` | SDK surface for process cartridges | 6 | `src/sdk.rs`, `src/tests/fixtures/rpc_fixture.rs` |
| `specs/spec04.md` | Lua, socket subscribe/resume, CLI publish/follow, end-to-end probe | 8 | `src/context.rs`, `src/socket.rs`, `src/main.rs` |

## Scores

complexity: 36
blast-radius: mid
workflow: probe-then-spec

**Complexity 36/100.** One new module plus a frame on every existing surface
(wire link, SDK, Lua ctx, socket, CLI) and ten tests; every piece is a
mechanical extension of an existing registry, but the determinism rule
(membership changes and their envelopes under one lock) and the resume path
are the parts that can be gotten subtly wrong.

**Blast-radius: mid.** `Runtime::fail` and `emit`'s listener contract changed
semantics (errors now land on the failing cartridge's channel; a failing
listener fails its own fiber), so any other consumer of emit-failure behavior
inherits a change; everything else is additive surfaces over new code.

**Footprint union:** `src/stream.rs`, `src/lib.rs`, `src/tests/stream.rs`,
`src/runtime.rs`, `src/cartridge.rs`, `src/lua.rs`, `src/sdk.rs`,
`src/tests/fixtures/rpc_fixture.rs`, `src/context.rs`, `src/socket.rs`,
`src/main.rs`.

## Evidence

- `cargo test --offline --lib` → **85 passed, 0 failed** (baseline 75), three
  consecutive green runs; `cargo clippy --offline --all-targets` clean.
- End-to-end probe (`probe/end-to-end.sh` from the lane): builds the real
  binary, starts a daemon over a throwaway profile, and a `follow build`
  client sees three ordered envelopes — its join (seq 1), the CLI publish
  (seq 2), and the Lua cartridge's publish from `send build 2` (seq 3):
  ```
  {"channel":"build","event":{"ch":"build","data":{"id":1},"from":"socket","kind":"subscribe","seq":1}}
  {"channel":"build","event":{"ch":"build","data":{"from":"cli","step":1},"from":"socket","kind":"data","seq":2}}
  {"channel":"build","event":{"ch":"build","data":{"from":"builder","step":2},"from":"builder","kind":"data","seq":3}}
  ```

## Implementer run (vega, re-verification after the lane healed onto main)

All four specs' boxes ticked **as they closed**, against the lane
(`.pearde/.lanes/the-telemetry-channels`, branch `lane/the-telemetry-channels`,
which had already merged `main` — the analyst's 85 grew to 106 tests from the
other lanes' work).

- spec01: all six boxes closed — `cargo test --lib tests::stream::` →
  `10 passed; 0 failed` (then 11 after the new test below).
- spec02: both named boxes closed —
  `tests::stream::a_failing_listener... ok` (1 passed) and
  `tests::stream::a_process_cartridge... ok` (1 passed). **The spec's verify
  paths said `tests::stream::tests::…`, which matched 0 tests** (the module
  path is `tests::stream::…`); the paths are corrected in `spec02.md`/`spec03.md`.
- spec03: box 2 (idempotent subscribe) had **no test** exercising the repeat —
  the guard (`link.sub_id` early-return, `cartridge.rs:480`) was read, but a
  box is ticked only for a check actually run, so this worker added
  `tests::stream::a_repeat_subscribe_spawns_no_second_pump` (inside
  `src/tests/stream.rs`, in the footprint union): one join in the log, one copy
  of a publish, zero duplicates → `ok`. Boxes 1 and 3 closed by the wire test,
  which asserts `echo_seqs == [base+1, base+2, base+3]` and reads channel
  frames and echoes in either order.
- spec04: the three test boxes closed in the same run; the probe box closed —
  `sh probe/end-to-end.sh` → the three envelopes quoted above, again live.
- Repo gate: `cargo test --offline --lib` → **106 passed, 0 failed**.
  `cargo clippy --offline --all-targets` → **1 error, outside this footprint**
  (see Failure below); clippy over this footprint's own surface is clean.

## Failure

**`cargo clippy --offline --all-targets` is red on this lane — one error, not
from this PRD.** `src/tests/wire.rs:163` (`host.call(&key.to_owned(), …)`,
clippy `unnecessary_to_owned`, `-D warnings` → the lib-test target does not
compile under clippy). `src/tests/wire.rs` was added whole by the-wire's commit
`ab7930e` ("work in progress at collect") and arrived in the lane through the
merge of `main` (`4eb62c0`); it is in no spec's footprint here, so per the brief
it is reported, not fixed — the-wire's worker should drop the `.to_owned()`
(the same lint family was already fixed in `sdk.rs` on this lane at `dacd08e`).
Everything in this PRD's footprint is clippy-clean; the lib suite passes.

## Findings

- **The record tooling exists.** the-wire's report claimed
  `resources/knowledge.py` was missing; it is at
  `/Users/feb/dev/infra/pearde/resources/{knowledge,workflows}.py` (found via
  the `pearde` launcher) and this pass queried and wrote to it five times.
- **Standing semantic correction (in-footprint, verified):** `emit` used to
  fail the *emitter's* fiber when a listener raised; it now fails the
  listener's fiber (`listeners()` returns `(Uid, Listener)`), so the error
  event names the party that broke.
- **Retention is unbounded in memory.** The in-memory log lives for the
  daemon's lifetime; the PRD puts retention out of scope, so this is a known
  shape, not a defect — but a long-lived daemon with a hot channel grows
  without bound until someone speccs eviction.
- **`zirkle tail` does not see channel frames** (it reads the outbox);
  `zirkle follow <ch>` is the channel reader. Nothing needed changing —
  named so nobody goes looking for channels in `tail`.
- **Probe hazards on the record:** a cargo build started under `.pearde/prds/`
  resolves up the directory tree to the main repo's Cargo.toml and can poison
  a shared target dir with a foreign binary; and a profile's `init.lua`
  entries resolve `path` against `--dir` (default `builtin` relative to cwd),
  not against the profile directory — the probe daemon needs
  `--dir "$PROBE"`. Both cost this run a red herring; both are on the record
  (`[[260912-5cc0]]`).
- Nothing out of scope was edited; no frontmatter touched; probe code stays
  uncommitted at `prds/the-telemetry-channels/probe/`.

## Route

Workflow `probe-then-spec` (its `## Use when` fits: a surface whose contract
could only be tested by building it), as run:

| step | did | notes |
|---|---|---|
| read-the-contract | yes | library atomic |
| query-the-record-first | yes | record tooling found and used |
| attempt-the-build | yes | library atomic |
| separate-the-machine-failure-from-the-crate | yes | the stale-binary red herring |
| record-what-the-build-learned | yes | 3 remember entries this pass |
| write-the-specs | yes | four specs, sum 36 |

### atomic drive-the-follow

A person-facing surface (here `zirkle follow`) is a guess until the real
binary runs it against a real daemon: build the lane's binary in a
probe-dedicated target dir, start a daemon over a throwaway profile with
`--dir "$PROBE"` so the profile's bare Lua entries resolve, drive it with a
second client, and print what that client received — the probe's stdout is
the spec's evidence.
