---
kind: work
description: "Agents build context as they go from memory, file access and the memo system; many dispatched agents share one living record"
status: blocked
owner: "sys-opus-2026-09-12/implementer-context-is-living-not-per-session"
level: 9
priority: P1
estimate: 2h
---

# context-is-living-not-per-session

Blocked: boxes 1-5 are observed green on the renamed tree and `just check` is
exit 0; box 6 needs `just test`, whose one remaining failure is
`test_router.py::test_a_launched_agent_reaches_the_models_through_the_proxy`,
failing because
`builtin/memory/.zirkle/memos/research/the-watcher-stores-an-empty-body-as-a-document.md`
has invalid frontmatter (`mapping values are not allowed in this context at line
3 column 155`). That memo lives in the separately versioned `builtin/memory`
repository this lane must not write to. Fix its frontmatter there, then one
`just test` closes box 6.

## Outcome

Context is a living record, not a per-session silo. Whatever dispatches an
agent — the user's shell, another agent, a lane — the agent builds its context
as it goes from the same three sources: the memory bank, file access, and the
memo system. A session's transcript is a fact source that feeds the record,
not the container the agent lives in.

Many, many agents can be dispatched, and they work together because they read
and write the same living context: one agent's correction, decision or finding
is the next agent's context, whatever session dispatched it. Orchestration
stays in the harness; the record is the shared medium.

User direction, 2026-09-12: "the context is essentially a living context so
it's not a per-session information context. Rather it is that the agents only
use the memory, file access, and the memo system to build their context as
they go. We can dispatch many, many different agents but everything is
orchestrated and working together."

## Approach

Files:

- `core/tests/test_living_context.py` — the probe, committed on lane
  `work/context-is-living-not-per-session` (commits `ff142d8`, `11f60e0`).
  Four cases, each dispatching fresh `zirkle run` processes so nothing is shared
  between them but the record, the bank and the filesystem.
- No production code changes. The record machinery already carries the
  Outcome: atomic validated writes with `expected_revision`, the cross-process
  lock, the usage resolver, the memory bank's per-profile store, and the
  harness's live system composition. The probe is the check.

Steps, from the lane:

1. `just memory-build` — the memory workspace is a separate Cargo workspace, so
   `cargo build --workspace` never produces `target/debug/memory`. Without it
   the fixture profile's `memory-tool` fiber stays `Inactive` with no error and
   `agent` never activates, and
   `zirkle tests::profile::native_system_memos_drive_live_harness_projection`
   fails with "agent not active". Measured: red without it, green with it.
2. `cargo build --quiet --workspace --bins` — the probe dispatches the real
   cartridge executables, and a stale `target/debug/pty` makes unrelated
   profile tests fail.
3. `python3 -m unittest discover -s core/tests -p test_living_context.py`
4. `just check`
5. `just test`
6. Tick the boxes below from the observed output.

What the probe already did — all four cases pass on `11f60e0`:

- `test_fact_crosses_sessions_without_a_copy`: session one writes an anchored
  note; a separate later dispatch resolves it by usage with `revision` and
  `why` provenance, a third reads the body back, a fourth composes `memo
  {op:"system"}`.
- `test_concurrent_agents_share_the_record`: three concurrent writers race one
  path under one `expected_revision` — exactly one wins, every loser fails with
  an explicit retryable `revision conflict` / `busy`, and the path still reads
  back as one valid body; then two concurrent agents write disjoint findings
  and a fresh resolve returns both.
- `test_a_fresh_session_names_the_sources_it_composed`: a session created in
  one dispatch, with no transcript, is composed by `harness {op:"inspect"}` in
  another. The reply names each source it drew on — `Source: composed memo
  template`, `Source: enabled system memos`, `Source: profile and session
  variables`, and `Source: <ws>/CLAUDE.md` — and renders a non-empty system
  prompt. This is the source-naming half of the Outcome, proved through the
  same surface `/context` shows a person.
- `test_memory_bank_carries_facts_across_sessions`: a temporary profile
  composes `memory` beside `memo`; one session ingests, another queries and
  gets `id` and `status` provenance. Skipped where no local embedding endpoint
  answers.

Run on the lane rebased onto the renamed trunk `5c2ee7d` — the `momo` → `zirkle`
rename `c7a0e0d` — with the probe ported to the new names (`zirkle run`,
`ZIRKLE_PROBE_*`, `.zirkle/`). Boxes 1-5 close on this output, on the renamed
binary, under `CARGO_TARGET_DIR=target/living-context-gate`:

```
The third source-check: a session with no transcript still composes a ... ok
test_concurrent_agents_share_the_record (test_living_context.LivingContext.test_concurrent_agents_share_the_record) ... ok
test_fact_crosses_sessions_without_a_copy (test_living_context.LivingContext.test_fact_crosses_sessions_without_a_copy) ... ok
test_memory_bank_carries_facts_across_sessions (test_living_context.LivingContext.test_memory_bank_carries_facts_across_sessions) ... ok

Ran 4 tests in 6.770s

OK
```

Box 5 did not skip: a local embedding endpoint answered, so the bank case ran
for real.

Step 1 of the Approach no longer suffices on the renamed tree. `just memory-build`
builds `--bin memory`, but `builtin/memory/cartridge.json` now names
`"binary": "memory_cartridge"` and `init.lua` returns
`zirkle.process("memory_cartridge")`, so the bank case fails with
`memory: process executable `memory_cartridge` was not found or is not
executable`. The `[[bin]] memory_cartridge` target exists; it just is not built.
Until the recipe catches up, the bank case needs

```sh
cargo build --manifest-path builtin/memory/Cargo.toml --bin memory_cartridge
```

beside `just memory-build`. Not fixed here: `builtin/memory` is a symlink to a
separately versioned repository mid-rename, and this lane writes nothing into it.

Observed, not settled here:

- `core/tests/test_development.py::test_cartridge_reload_and_failed_build_keep_the_surface`
  is flaky, on this lane and on the trunk at `92712f4` alike — seen failing two
  different ways (`2 != 1` alt-screen enters; "terminal condition not reached")
  and passing on the next run of the same binary. Not this memo's; re-run it
  rather than read it as a regression.
- The memory bank — one of the Outcome's three sources — is composed by no
  shipped profile and exposed through no agent tool (the default profile
  dispatches `tool.shell` and `tool.memo` only), so `harness inspect` cannot
  name it among the sources. The probe reaches the bank through a temporary
  profile instead. That wiring is
  [[the-memory-bank-is-wired-into-the-surface]]'s contract, and writing durable
  knowledge to it with anchors is
  [[context-corrections-carry-their-context]]'s; neither is specced here.
- The thirteen live lanes share one `target/` directory. A concurrent lane's
  build replaces a cartridge binary mid-run and fails unrelated profile tests
  with "process executable ... was not found or is not executable"; a full disk
  stops every gate. Re-run rather than read either as a regression.

Box 6 is the one box still open, and what holds it is outside this memo.

`just check` is green: exit 0 on the renamed tree, `memory-check` included.

`just test` reaches exactly one failure, and it is not this lane's:

```
Summary [  41.282s] 1337 tests run: 1337 passed, 17 skipped
Summary [  35.647s] 249 tests run: 249 passed, 1 skipped
FAIL: test_a_launched_agent_reaches_the_models_through_the_proxy
AssertionError: 'the provider answered' not found in '...
{"error":{"message":"cartridge memory: research/the-watcher-stores-an-empty-body-as-a-document.md: invalid frontmatter: mapping values are not allowed in this context at line 3 column 155","type":"proxy_error"}}'
```

Both cargo suites are fully green — memory `1337 passed`, zirkle `249 passed` —
`bun test` passes, `eval_compaction.py --check` prints `offline gate passed`,
and every other `core/tests/test_*.py` file passes on its own, `test_development.py`
included. The one failing memo is
`builtin/memory/.zirkle/memos/research/the-watcher-stores-an-empty-body-as-a-document.md`,
inside the separately versioned `builtin/memory` repository that every lane
symlinks and that another session is mid-rename in. Fixing it means writing
there, which this lane does not do. Box 6 closes with one `just test` once that
memo's frontmatter is valid.

Earlier gate noise, all cleared and recorded so it is not re-chased:

- `agent::loop` aborting with ``"`agent` is not provided"`` — a different test
  each run, always passing alone: another lane's `cargo build --workspace`
  replacing a cartridge binary in the shared `target/` mid-run. An isolated
  `CARGO_TARGET_DIR` cleared the class.
- `zirkle tests::profile::profile_is_path_entries_only_with_one_tools_table`
  failing `["harness", "harness.selftest", "harness.integration"] != ["harness"]`
  — the shared `target/debug/harness` built from a newer trunk than the lane's
  base. A rebase cleared it.
- `kern::e2e hub::vanished_root_reaper_*` failing on 30-35s wall-clock waits
  under load average 13, a different subset each run, while another lane ran the
  same three tests against the same per-user hub. Green now.
- `pty::process shell_serializes_input_and_only_cancels_the_matching_invocation`
  failing `Elapsed(())` once on that same load. Green now.

## Check

- [x] A note written by one dispatched session is resolved by usage in a
      separate later session, carrying `revision` and `why`, with no copy step.
- [x] Three concurrent writers against one path under one `expected_revision`
      leave exactly one exit-0 winner; every loser's error names `revision
      conflict` or `busy`, and the path afterwards reads back as exactly one of
      the written bodies.
- [x] Two concurrent agents' disjoint findings both land, and one fresh resolve
      returns both paths.
- [x] A session with no transcript is composed in a different dispatch, and the
      reply names `Source: composed memo template`, `Source: enabled system
      memos`, `Source: profile and session variables` and the workspace
      `CLAUDE.md`, with a non-empty rendered system prompt and no
      `projection_error`.
- [x] The memory bank ingested in one session is queried in another and returns
      the fact with `id` and `status` provenance (skips where no local
      embedding endpoint answers).
- [ ] `just check` and `just test` are green on the lane.

```sh
just memory-build
cargo build --manifest-path builtin/memory/Cargo.toml --bin memory_cartridge
cargo build --quiet --workspace --bins
python3 -m unittest discover -s core/tests -p test_living_context.py -v
just check
just test
```
