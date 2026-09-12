---
kind: work
description: "One ranking engine owns nodes, connections and observed use for tools, memos and routines"
status: done
level: 10
priority: P1

---

# tool-graph-engine-is-the-ranking-database

## Outcome

`builtin/toolgraph` is the ranking database's engine. Nodes are everything the
system can reach for — tools, memos, routines — each with a kind, a stable key
the caller uses to reach it, a description, and the situation phrases it
declares (`when:`). Edges carry the connections between nodes: a memo's wiki
links, the tool a dispatch ran, the usage a routine serves. Every observation —
dispatch, resolve and outcome — is counted per node with a stage weight (a use
and a confirmed success count, a bare surface does not), and one search ranks
nodes by match times observed standing, returning ranked entries with
descriptions.

The engine only has to stand up. Who upserts nodes and links edges belongs to
[[tool-dispatch-and-routines-are-graph-nodes]], and the resolver's move behind
it to [[the-resolver-runs-on-the-tool-graph-engine]].

## Approach

Files: `builtin/toolgraph/src/lib.rs` — all engine work, no other file.

Already built and on the trunk (commits 9fa45eb, 67b5148, 5c5b4b2):

- `Node` (kind, key, description, when, name, tags), field-weighted `search`
  (name 3, when 3, tags 2, description 1) times standing `1 + ln(1 + uses)`.
- `Edge { from, to, kind }` and `Graph` — nodes in a `BTreeMap` keyed by
  `node.key`, edges in a `BTreeSet`, so a repeated edge is one edge.
- `Graph::upsert(node)` grows the node set in place: a key already present is
  replaced, so a tool that re-registers or a memo that was edited updates
  instead of doubling. Nodes and edges are derived state rebuilt per
  composition — the engine keeps no durable copy; the record and the
  inventories stay the sources of truth.
- `Graph::link(edge)` tolerates endpoints not yet upserted (a dispatch may name
  a tool to come); `edges_of(key)` reports edges in either direction,
  `neighbours(key)` only the far ends that are present.
- `Graph::search(query, counts)` delegates to the free `search`;
  `graph_search_keeps_the_seed_ranking` re-proves the seed ranking through it.
- Where the counts live, settled: the memo resolver's observation journal
  (`.zirkle/resolver/events.jsonl`, format owned unchanged by
  [[the-resolver-runs-on-the-tool-graph-engine]]) is the durable count store,
  read through `counts_from_journal`. The engine owns no second tally. The
  journal's bounded retained window (2048 events, oldest dropped first) is the
  standing decay — observations that fall out stop counting; no decay in the
  engine. The stage vocabulary is the resolver's own `surfaced` / `used` /
  `outcome` (`builtin/memo/src/resolver.rs`), so no third stage is invented.

What the analyst probe on lane `work/tool-graph-engine-is-the-ranking-database`
found and fixed (commit b8e2413): the landed engine fails `cargo fmt --all --
--check`, the first step of `just check`, and is the only offender in the
workspace. The earlier Check ran only crate-scoped `cargo test` and `cargo
clippy`, so a red repo gate landed under a ticked box. The fix is one
`cargo fmt -p toolgraph`, committed on that lane; the six unit tests, clippy and
the real-record smoke were re-run green over the formatted source.

Remaining, in order:

1. `just land tool-graph-engine-is-the-ranking-database` from
   `/Users/feb/dev/sys` — the lane holds only the formatting commit.
2. From the trunk, run the `sh` block below; it is the Check, and it starts with
   the repo-wide fmt gate rather than a crate-scoped subset.
3. Change engine code only if a box stays red; keep it inside `lib.rs`, keep the
   six unit tests passing unmodified, and keep the doc comments current.

## Check

- [x] `cargo fmt --all -- --check` exits 0 from the trunk — the gate `just check`
  runs first, and the one the engine landed red.
- [x] `cargo clippy -p toolgraph --all-targets -- -D warnings` is clean.
- [x] `cargo test -p toolgraph` passes all six unit tests unmodified:
  `search_returns_kinds_with_descriptions`, `observed_use_outranks_equal_match`,
  `journal_counts_weight_stages`, `upsert_grows_the_node_set_in_place`,
  `edges_connect_present_nodes_and_tolerate_ones_to_come` and
  `graph_search_keeps_the_seed_ranking`.
- [x] The real-record smoke `real_record_search` passes over the live
  `.zirkle/memos` and resolver journal, printing ranked hits across kinds.

```sh
cd /Users/feb/dev/sys \
  && cargo fmt --all -- --check \
  && export CARGO_TARGET_DIR=/Users/feb/dev/sys/target/toolgraph-gate \
  && cargo clippy -p toolgraph --all-targets -- -D warnings \
  && cargo test -p toolgraph \
  && ZIRKLE_RECORD=/Users/feb/dev/sys/.zirkle/memos cargo test -p toolgraph -- --ignored --nocapture
```

estimate: 2h

## Result

Delivered. The engine landed on the trunk as `9fa45eb`, `67b5148` and `5c5b4b2`;
the formatting the repo gate demands landed as `9380d89` from lane
`work/tool-graph-engine-is-the-ranking-database`, which closed the red gate that
the first pass ticked over.

Evidence (coordinator, 2026-09-12, trunk at `9380d89`): `cargo fmt --all --
--check` exits 0 with no diff; `cargo clippy -p toolgraph --all-targets -- -D
warnings` finishes clean; `cargo test -p toolgraph` reports `ok. 6 passed; 0
failed; 1 ignored`; `ZIRKLE_RECORD=.zirkle/memos cargo test -p toolgraph -- --ignored
--nocapture` reports `test tests::real_record_search ... ok` and prints ranked
hits across kinds over the live record (`8.00 [work] work/gates-run-in-a-lane.md`,
`4.00 [routine] routine/gh-run.md`).

Edge population from wiki links and dispatch stays with
[[tool-dispatch-and-routines-are-graph-nodes]], as the Outcome records.
