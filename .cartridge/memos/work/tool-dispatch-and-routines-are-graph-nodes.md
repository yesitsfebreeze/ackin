---
kind: work
description: "Tool inventory and routines are graph nodes, and every dispatch and use is an observation the graph counts"
status: done
level: 10
priority: P1
needs: ["[[tool-graph-engine-is-the-ranking-database]]"]
---

# tool-dispatch-and-routines-are-graph-nodes

## Do

The graph's nodes include the live surface: every tool the host inventory
serves (its describe description is the node's description) and every routine
memo in the record, with the usage phrases they declare. Every dispatch,
resolve and routine use is recorded as an observation the engine counts, so
how often a thing is used enforces its ranking. The memo record's resolve and
observe path feeds the engine rather than a private tally.

Sibling surfaces sit on the same ranked graph, not beside it:
[[the-memory-bank-is-wired-into-the-surface]] may hang memory retrieval on it.
Keep the node model open to further kinds instead of claiming another
cartridge's store.

## Spec

Files: `builtin/toolgraph/src/surface.rs` and `builtin/toolgraph/src/lib.rs`
(one `pub mod surface;` line), `builtin/memo/src/resolver.rs`,
`builtin/memo/tests/journal.rs` (one added test), `builtin/agent/lib.rs`,
`builtin/agent/model_loop.rs`, `builtin/agent/main.rs`,
`builtin/agent/Cargo.toml`, `builtin/agent/tests/loop.rs` (fixture stubs plus
one added test).

The probe already built (commits b24e6fe, 06ca408, ba23ef3, all in the lane):

- `builtin/toolgraph/src/surface.rs` — the live surface grown into the graph:
  `tool_node(key, descriptor)` (kind `tool`, the dispatch key as `node.key`,
  the describe description as the node's description, the model-facing name in
  `node.name`), `routine_node(path, memo)` (kind `routine`, the memo path as
  key, the memo's description, the union of its `uses` `when:` phrases as the
  node's situation phrases), and `compose(descriptors, routines, journal)`
  which upserts both into a `Graph` and reads standing with
  `counts_from_journal` — the journal, never a private tally. Entries that
  fail their upstream shape (descriptor without `name`, memo that is not an
  object) are skipped; the callers validate them. Tests prove tool nodes carry
  the describe description, routine nodes carry the declared usage phrases,
  compose ranks a used tool above its equally matched sibling and surfaces a
  routine through its `when:` phrase, and malformed entries are skipped.
- `builtin/memo/src/resolver.rs` `observe` — tool observations are journalled
  beyond the record: a path shaped as a tool key (`tool.` prefix, no `/`,
  alphanumeric plus `.`, `-`, `_`, at most 256 bytes) skips the memo-path and
  identity checks; everything else (stage, hex revision, size, outcome parent
  and evidence rules) is unchanged, and the journal format gains nothing.
  Memo-path observations validate exactly as before. New test
  `tool_observations_land_in_the_journal_beyond_the_record` proves the
  surfaced-to-used chain and the refusals; the existing suite passes
  unmodified.
- `builtin/agent` — the dispatch is the observation. `model_loop` digests each
  describe descriptor (`sha2`, added to the agent's dependencies) into the
  revision the observation cites; `Run::tool` observes `used` on the tool key
  right after policy allows and `outcome` (`success`/`failure`/`cancelled`)
  once the result is in, anchored on the used event's id, through a
  best-effort `observe_tool` that never fails the dispatch it records
  (yolo's skip at the memo service returns no `saved` and is treated as
  skipped). The agent injects `memo` and calls the record's native observe
  envelope (`cwd`, `op`, `event`, `context` with a distinct origin call per
  stage, since the journal id is the origin digest). The loop-test fixture
  gained a record stub, and the new test
  `a_dispatch_and_its_outcome_are_recorded_observations` proves used and
  outcome land with path, usage, descriptor-digest revision, outcome verdict,
  parent and evidence.
- Live proof: the lane's `momo run memo` journalled a `surfaced` and then a
  `used` event for `tool.shell` into a fixture record, and the record's
  `resolver` report returned them (`counts {"surfaced":1,"used":1}`).

Settled by building: resolve and routine use are recorded through the record's
existing observe path — a routine is an ordinary memo, so its use is an
explicit observe of the memo path and usage exactly as the resolver tests do
it, and the same journal is what the engine counts. The record does not
auto-journal resolves: that would change the resolve op's behaviour, which
[[the-resolver-runs-on-the-tool-graph-engine]] forbids, and the record's
standing attribution stays explicit caller evidence.

Remaining for the implementer (gates and landing only):

1. `cd /Users/feb/dev/sys/.claude/worktrees/tool-dispatch-and-routines-are-graph-nodes`
   (the lane; it already holds all three probe commits).
2. Run the crate gates in the `sh` block below.
3. Run the record-level gates:
   `CARGO_TARGET_DIR=/Users/feb/dev/sys/target/dispatch-nodes-gate` with
   `proxy` built into it first, then `just check` and `just test` from the
   lane.
4. Optionally re-run the engine's real-record smoke
   (`MOMO_RECORD=/Users/feb/dev/sys/.momo/memos cargo test -p toolgraph --
   --ignored --nocapture`) to see the live surface rank.
5. Fix nothing unless a gate fails; a fix stays inside the files above and
   keeps the existing tests unmodified. Commit each coherent change.

## Check

- [x] `cargo test -p toolgraph` passes, including the surface tests
  `tool_nodes_carry_the_describe_description`,
  `routine_nodes_carry_the_declared_usage_phrases`,
  `compose_grows_the_live_surface_and_counts_observations` and
  `compose_skips_malformed_surface_entries`, with the seed engine tests
  unmodified.
  Evidence: `test surface::tests::tool_nodes_carry_the_describe_description ... ok`,
  `test surface::tests::routine_nodes_carry_the_declared_usage_phrases ... ok`,
  `test surface::tests::compose_grows_the_live_surface_and_counts_observations ... ok`,
  `test surface::tests::compose_skips_malformed_surface_entries ... ok`,
  `test result: ok. 10 passed; 0 failed; 1 ignored` (seed engine tests among
  them, unmodified).
- [x] `cargo test -p memo_cartridge` passes, including
  `tool_observations_land_in_the_journal_beyond_the_record` (a tool
  observation is journalled, its use anchors on its surfaced parent, and
  malformed tool paths and undeclared memo paths are refused), with the
  existing resolver and journal tests unmodified.
  Evidence: `test record::resolver::tests::tool_observations_land_in_the_journal_beyond_the_record ... ok`,
  `test result: ok. 30 passed; 0 failed` (existing resolver and journal tests
  pass unmodified alongside it).
- [x] `cargo test -p agent` passes, including
  `a_dispatch_and_its_outcome_are_recorded_observations` (a dispatched tool
  yields one `used` observation with a descriptor-digest revision and one
  `success`-verdict `outcome` anchored on it, and no `surfaced` events).
  Evidence: `test a_dispatch_and_its_outcome_are_recorded_observations ... ok`,
  `test result: ok. 13 passed; 0 failed` (tests/loop.rs). First parallel
  three-crate run flaked two loop tests under load; both passed alone on the
  same build and the full command passed on rerun.
- [x] `cargo clippy -p toolgraph -p memo_cartridge -p agent --all-targets`
  is clean (warnings denied).
  Evidence: `Finished \`dev\` profile [unoptimized + debuginfo] target(s) in 3.52s`
  with no warnings emitted for the three crates.

```sh
cd /Users/feb/dev/sys/.claude/worktrees/tool-dispatch-and-routines-are-graph-nodes && CARGO_TARGET_DIR=/Users/feb/dev/sys/target/dispatch-nodes-gate cargo test -p toolgraph -p memo_cartridge -p agent && CARGO_TARGET_DIR=/Users/feb/dev/sys/target/dispatch-nodes-gate cargo clippy -p toolgraph -p memo_cartridge -p agent --all-targets
```

estimate: 2h

## Result

status: done

Blocked note struck at landing: the contested momo→zirkle rename that blocked
the shared gates has now landed on trunk (the resolver lane's read-only
cartridge merge and the zirkle rename both shipped ahead of this one). Rebased
onto 6f44f04 and re-ran the full gates on the lane: `just check` green, `just
test` green (nextest 1337 + 255 suites, one rotated inventory flake that
passes in isolation, and the `proxy` bin built into the fresh target dir the
python suites need). The lane lands with the same surface delivered below.

Delivered: the probe's whole surface is landed and green on the lane at trunk
tip a5d4553 (lane tip 697b643). Tool and routine nodes grow into the graph from
describe and the record (`builtin/toolgraph/src/surface.rs`), tool dispatches
and outcomes journall through the record's observe path
(`builtin/memo/src/resolver.rs`, `builtin/agent/{lib,model_loop,main}.rs`), and
all four Check boxes close with quoted evidence above, re-observed after the
final rebase. The engine's real-record smoke passed (361 nodes ranked from the
live record).

Repository-checks did not fully run, through no defect in this memo's scope.
The contested momo->zirkle rename breaks the shared gates for every lane until
it resolves:

- `just check`/`just test` fail first at `memory-check`/`memory-test`: the
  gitignored `builtin/memory` symlink lends the trunk checkout's memory
  workspace, which depends on `zirkle = { path = "../../core" }`, while the
  trunk branch's core crate is still named `momo` (`error: no matching package
  named \`zirkle\` found`).
- While the trunk branch briefly carried the rename (dffa688, since reset),
  `cargo clippy --workspace --all-targets` failed at the trunk tip itself
  (`core/tests/profile.rs` still `include_str!`s `.momo/default/init.lua`,
  absent from that tree) and the core python suites failed on the same
  `.momo` paths; those failures were observed, not caused, by this lane.
- The memo service fails closed on both the shared checkout and this lane
  (`no daemon for .momo/default`), so these ticks were written by direct edit
  into the lane's record copy per the coordinator's rule; the shared
  checkout's `.zirkle/memos` copy of this memo already carries the same four
  ticks (written before the contested state was known). The coordinator
  reconciles the copies with the record decision.

What clears it: the rename decision landing (memory workspace dep name and
core crate name agreeing on every lane, `.momo` path references in
`core/tests/*.py` settled) — then a rebase and a `just check` + `just test`
rerun in this lane. Every gate step that is runnable today passed in the lane:
workspace `cargo fmt --all -- --check`, the ui check and 20 ui tests, workspace
doc tests, the tools python suite (7 OK), the four Check boxes, and
`cargo clippy -p toolgraph -p memo_cartridge -p agent --all-targets`.

Lane commits (on trunk tip a5d4553): 7b4ce0a/34c73ae-successor probes for
surface, resolver journal and agent dispatch observations, the sha2 lockfile
commit, and two rustfmt passes.
