---
description: "The memory bank is composed by no shipped profile and exposed through no agent tool"
estimate: "2h"
kind: "work"
level: 10
needs: ["[[context-corrections-carry-their-context]]"]
priority: "P1"
status: "done"
---


# the-memory-bank-is-wired-into-the-surface

## Do

Observed while speccing [[context-is-living-not-per-session]]: the memory bank
exists (store, snapshot, ingest, query) but no shipped profile composes the
`memory` cartridge, and the agent's dispatch list is only `tool.shell` and
`tool.memo`. The living-context direction — agents build context from memory,
file access and memos — cannot hold while memory is unreachable from the
surface.

Wire it: the default (and terminal) profiles compose `memory` where it
belongs; an agent tool key exposes its query/ingest under the same policy as
the other tools; a fixture proves a fact lands in the memory bank and is
recalled by a later agent turn. Whether this belongs to
[[context-corrections-carry-their-context]] (which stores corrections in the
bank) or stands alone is a spec decision — the corrections lane already
touches the same surface.

## Spec

Stands alone: [[context-corrections-carry-their-context]] is done and stores
corrections in the memos with the anchor as provenance, adding no bank write
path; this memo wires the bank itself and nothing of the corrections surface.

The adapter lives in the zirkle workspace, not the nested memory repository: the
memory workspace is separately versioned and its trunk copy carries another
lane's uncommitted work, while the tool envelope is a zirkle-surface concern
(memo, pty and fs each own their copy of it). A Lua cartridge cannot forward a
call to a sibling service, so the adapter is a small process cartridge.

There is no terminal profile in the tree; the bank is composed by `.zirkle/proxy`
already (external-agent recall) and keeps its exact tool set there — the Do's
wiring target that exists is the default profile, and that is what this spec
changes.

**Files**

- `builtin/memory-tool/` (new: `Cargo.toml`, `cartridge.json`, `init.lua`,
  `src/main.rs`) — adapter process cartridge, injects `memory`, provides
  `tool.memory`: `describe` (name `memory`, schema `op` query|ingest with
  `text` required, `k`, `raw`), `call` forwards to the memory service
  (sync ingest by default), `cancel` answers (engine calls are synchronous).
  Malformed context or op fails closed; an ingest the engine could not place
  (`status` not `committed`, e.g. dead embedder) is an explicit tool error,
  never a silent success.
- `Cargo.toml` — workspace member `builtin/memory-tool`.
- `.zirkle/default/init.lua` — entries `{ id = "memory", path = "memory" }` and
  `{ id = "memory-tool", path = "memory-tool" }` after `policy`; the one tools
  table gains `tool.memory`.
- `.zirkle/default/config.lua` — `memory = { dir = ".zirkle/memory" }` (endpoints
  default to a local Ollama).
- `builtin/policy/init.lua` — memory branch: `query` allow, `ingest` ask,
  anything else on the bank denies.
- `core/tests/profile.rs` — the boot smoke stubs `memory` and `memory-tool` as
  Lua cartridges (surface keys only); manifest assertions now expect the two
  entries and tools `["tool.shell","tool.memo","tool.memory"]`; the
  `shell_timeout_and_memo...` test strips the memory entries instead of
  asserting the folder's absence, keeping the boots-without-memory property.
- `core/tests/policy.rs` — memory decision split assertions in
  `policy_defaults_distinguish_reads_from_writes`.
- `core/tests/test_memory_tool.py` (new) — hermetic fixture (fake local
  embedding server, temp profile composing the real `memory` and `memory-tool`
  binaries through `--dir builtin` path entries): a fact ingested through
  `tool.memory` in one session is recalled with `id`/`status` provenance in a
  later one; a dead embedder surfaces as an explicit tool error; an unknown op
  fails.
- `justfile` — `test` builds `memory-tool` and the memory binary before the
  python tests.

**Probe (commits 07a6129, 13854cc on the lane branch)**: all of the above is
built and green on the lane — profile suite 10/10 (`cargo nextest run -p zirkle
-E 'test(profile)'`), policy test, fixture 3/3, `cargo clippy --workspace
--all-targets -- -D warnings` clean, fmt clean. Two batch failures were shared
target contention (both pass alone and in a rerun); the trunk `target/debug`
was trimmed of `incremental.kache-auto` and `examples` to free disk.

**Left for the hand**: the full gates on the lane (`just check`, `just test`,
`just all`), then tick the boxes. Note: `memory-check` runs over the nested
memory repository, whose trunk copy carries another lane's uncommitted work —
a failure there is pre-existing, not this lane's. Also note: a second writer
touched this lane mid-probe (a duplicate fixture appeared at
`builtin/memory-tool/tests/` and `profile.rs` was edited externally); the
duplicate was removed in 13854cc — the fixture lives only in `core/tests/`.

**Steps**

1. `cd /Users/feb/dev/sys/.claude/worktrees/the-memory-bank-is-wired-into-the-surface`
2. `export CARGO_TARGET_DIR=/Users/feb/dev/sys/target/memory-wiring-gate` and
   `cargo build -p proxy` into it before `just test` (test_router.py needs the
   proxy binary).
3. `just check`
4. `just test`
5. `just all`

## Check

- [x] A shipped profile composes `memory` and the agent can query and ingest
      through a tool key in the default profile.

      `.momo/default/init.lua` carries the `memory` and `memory-tool` entries and
      `tool.memory` in the tools table; `core/tests/profile.rs` asserts the
      composed entries, the agent dispatch list `["tool.shell","tool.memo","tool.memory"]`
      and the bank folder, and `builtin/policy/init.lua` splits query/ingest.
      Evidence (rebased lane, `cargo nextest run -p momo -E 'test(profile) or
      test(policy_defaults)'`):

      ```
      Summary [  25.036s] 12 tests run: 12 passed, 55 skipped
      ```

- [x] A fact recorded in one session is recalled through the tool in a later
      session; disabled or unconfigured memory degrades explicitly, never
      silently.

      `core/tests/test_memory_tool.py` composes the real `memory` and
      `memory-tool` binaries in a hermetic profile: ingest in one session,
      recall with `id`/`status` provenance in a later one; a dead embedder is
      an explicit tool error; malformed input fails closed. Evidence:

      ```
      Ran 3 tests in 3.362s
      OK
      ```

      Without memory composed, the surface degrades explicitly: `service
      `tool.memory` unavailable: memory failed to load; memory-tool inactive
      waiting for memory` (observed while the contested rename broke the entry,
      before the lane-local snapshot).

```sh
cd /Users/feb/dev/sys/.claude/worktrees/the-memory-bank-is-wired-into-the-surface
export CARGO_TARGET_DIR=/Users/feb/dev/sys/target/memory-wiring-gate
cargo build -p proxy
just all
cargo nextest run -p zirkle -E 'test(profile) or test(policy_defaults)'
python3 -m unittest discover -s core/tests -p test_memory_tool.py
```

## Result

Done on the rebased lane (trunk tip a5d4553; lane tips ef53781, 364dc58 —
content of probe commits 07a6129/13854cc). The default profile composes
`memory` (bank at `.momo/memory`) and `memory-tool`, whose `tool.memory` key
exposes query (allow) and ingest (ask) under the shared policy split.

Gates on the lane, all green in one pass each:

- `just check` → `EXIT=0` (fmt, clippy incl. the memory workspace, bun check).
- `just test` → `EXIT=0`: memory workspace `Summary [  42.221s] 1337 tests
  run: 1337 passed, 17 skipped`; momo workspace `Summary [  14.249s] 229 tests
  run: 229 passed`; bun `20 pass / 0 fail`; tools fixture `Ran 7 tests ... OK`;
  every `core/tests/test_*.py` OK (router, shell, ui, development, memory_tool).
- `just all` after the rebase → `EXIT=0`: memory `1337 passed`, momo
  `Summary [  23.825s] 249 tests run: 249 passed, 1 skipped`, python suites OK.
- Memo check commands verbatim: `cargo build -p proxy` finished;
  profile/policy `12 passed`; fixture `Ran 3 tests ... OK`.

Environmental notes for the record: the contested `momo`→`zirkle` rename (staged,
uncommitted on trunk) made the nested trunk memory workspace expect `zirkle`
against the lane's `momo` host, breaking `memory-test`, the memory binary build
and every memory-composing fixture from any lane. The trunk copy was left
untouched; the lane's gitignored `builtin/memory` symlink was replaced by a
lane-local snapshot of the same working tree with the three `zirkle` crate
references (init.lua, one dep line, one `use`) renamed back to `momo`, plus its
own git repo so its self-gates can run. First run of the full suite also hung
22 minutes on the pre-existing `foreground_missing_or_failed_service_still_disposes`
(four lanes running gates at once); it passed alone in 0.036s and in both later
full runs. `just check`/`just test` were re-observed green after the rebase via
`just all`.
