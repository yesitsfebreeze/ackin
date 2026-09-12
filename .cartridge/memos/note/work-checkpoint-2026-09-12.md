---
kind: note
description: Current native work pass ownership and integration evidence
date: "2026-09-12"
---

# Work checkpoint — 2026-09-12

Work runs on `terminal-integration-and-native-memos` from `71a10a8`.
The native memo service supplies the board; the Kern planner is unavailable.
Current work uses Outcome/Check and active/owner. [[work-checkpoint]] retains
previous sessions' history. Record changes belong to the coordinator; code
workers use isolated lanes and do not land them.

## Current pass

Active workers: program-map, native-profile and rpc-contracts own their named
worktrees under `.claude/worktrees/`. Record reconciliation is complete: 18
validated memo changes, zero missing/cyclic/off-axis work edges in its composed
scan. Compatible grid and collaboration work remains open. Prior session-owned shell follow-up code was reviewed and preserved as
`1b6e21c`; audit/work records as `438cf57`. Other worktrees are preserved. No existing native active claim was found at scan.

The composed board includes cartridge-owned history. Initial scan counts and
axis analysis are recorded below. Old sidebar/gutter and bottom-chrome removal
contracts are being reconciled with the user's latest-tool footer and full
transcript direction before UI dispatch. Compatible terminal-grid work remains
subject to its existing contract.

Next: collect probes and evidence, integrate finished lanes serially, dispatch
newly ready program debug and bootstrap work. A clean checkpoint or committed
probe is not completion. [[runtime-stays-small-and-provable]] owns the new work.

Scan: 62 work items; 21 open/active; 0 off-axis.

## Integration constraints found

Default profile removal must also restrict foreground builds to the profile's
loaded cartridge sources, including nested Lua composition; otherwise memory
still builds. The native-profile worker owns that change.

The optional memory checkout is based on upstream commit
`58850707f37f7cd10939b742c4c9898e84552193` but its cartridge adapter and engine
entrypoint include local/untracked changes. A bootstrap must preserve and
reproduce the required adapter; an upstream SHA alone is insufficient. The
nested repository remains untouched by this pass.

## User-directed closeout

The user requested committing current work and merging it into `main`.
New dispatches are paused. Native-profile commits `e508e70` and `968927c` are
integrated; their full `just all` gate is running. Program-map and RPC workers
are checkpointing committed progress and reporting remaining scope. The empty
fresh-bootstrap lane was created but no worker started; its work returns open.
Only finished, reviewed scope will be merged; unfinished probes remain on their
branches. `main` is an ancestor of the integration branch, so no rewrite or
conflict resolution is currently needed.

## sys-opus 2026-09-12

Scan was rebuilt by hand: this workspace has no planner CLI, so the board is
read by parsing work frontmatter across `.zirkle/memos/work/` and every enabled
cartridge's `.zirkle/memos/work/`. 75 work memos, 29 open or active.

Three parent memos carried a duplicated `subwork:` key — repeated frontmatter
edits appended a second block instead of merging — so their first child fell
out of every composed board and the axis walk reported children as off-axis.
Merged in `fd57f82` (`the-record-feeds-the-agent`, `the-terminal-is-drawn-from-pty`,
`the-work-can-be-proven`). Off-axis count is now zero. One dangling reference
remains, `build-ratatui-element-framework` needing `tui-is-a-ratatui-element-framework`,
which no memo defines; it belongs to the `ui` cartridge record.

Claimed and dispatched in `92712f4`, nine lanes, none of them the two memos this
session found already owned by `codex-work-2026-09-12`
(`rpc-contracts-run-across-rust-lua-and-bun`, `terminal-profile-starts-only-needed-services`):

- Implementers: `cartridges-prove-themselves-at-registration`,
  `debug-mode-correlates-a-terminal-turn`, `pty-owns-the-terminal-grid`,
  `rolling-context-retains-decision-evidence`.
- Analysts: `check-boxes-lint-as-observable-behaviours`,
  `context-is-living-not-per-session`, `the-memory-bank-is-wired-into-the-surface`,
  `the-palette-and-exit`, `tool-graph-engine-is-the-ranking-database`.

Every worker commits on its own lane and lands nothing; landing stays serial and
with this coordinator. `pty-owns-the-terminal-grid` is the frontier that unblocks
`pty-encodes-input`, `the-ui-paints-the-grid` and the whole gutter/sidebar branch.

Next: collect each return, run `just all` on the combined result, land serially,
then re-scan for what the returns unblock.

### sys-6d pass, second half

Landed: [[rolling-context-retains-decision-evidence]] (compaction eval corpus,
rubric and offline gate, now the last step of `just test`) and
[[tool-graph-engine-is-the-ranking-database]]. Both lanes closed.

The tool-graph engine had landed from another pass with `cargo fmt --all --
--check` red under a ticked box — its Check ran only `cargo test -p toolgraph`
and `cargo clippy -p toolgraph`, never the repo gate. Fixed as `9380d89` and
closed with all four boxes observed from the trunk (`9081a47`). A crate-scoped
Check is not the repo gate.

[[debug-mode-correlates-a-terminal-turn]] returned FAILED from an implementer
(no Spec, no sh block — it was never specced), then SPLIT into four children.
Its probe found that `zirkle run` never served a socket — only `zirkle daemon` did —
so nothing reaching the foreground host through the visible shell had anything
to talk to, and a shell `zirkle call` silently loaded a second copy of the profile.
~14 lines in `core/main.rs` plus `core/tests/test_run_socket.py`, seen red then
green, on lane `work/debug-mode-correlates-a-terminal-turn` as `165cd6d`.
Analysts are on the two unblocked children.

Three sessions worked this tree at once (sys-6d, sys-fb, sys-de). Claims were
split by message: the memory bank went to sys-fb, the palette stayed here.
[[the-trunk-checkout-is-a-landing-pad]] records what the collisions taught.

Standing hazard: `.cargo/config.toml` points every worktree at one shared
`target/`, so a sibling lane's build hands `memo_cartridge` a stale `zirkle` rlib
and `cargo nextest run -p memo_cartridge` fails with `no method named inventory
found for &zirkle::sdk::Host` on untouched code; `touch core/sdk.rs` clears it.
Two independent probes hit it, and one filled the disk mid-pass. Worth its own
work memo.

Landing is blocked while another session's analyst has `builtin/ui` files
uncommitted on the shared checkout; `land` and `lane-rm` refuse for everyone
until it is clean.

### sys-6d landing wave

Landed and closed: [[check-boxes-lint-as-observable-behaviours]] (advisory
memo-level lint, `memo_cartridge` 29/29 re-verified on the trunk) and
[[pty-owns-the-terminal-grid]] at `cfe48c5` (`builtin/pty/grid.rs` owns the grid,
bounded frame versions, clamped scroll delta, one-step resize; 16/16 pty tests
green on the trunk). Both lanes removed.

`just land` refuses on any tracked change anywhere in the tree, which with four
sessions sharing this checkout is close to permanent. Where a lane's paths
provably do not intersect the dirty paths, `git rebase <trunk>` in the lane then
`git merge --ff-only` from the trunk does what `land` does and leaves the dirt
alone. The non-overlap check is what makes it safe; skipping it lands somebody's
mid-write record into an unrelated commit.

Specs returned and passed to implementers: [[context-is-living-not-per-session]],
[[the-palette-and-exit]], [[a-turn-carries-one-id-through-host-lua-and-bun]].
The turn-id probe found `Link::request` is the single place a wire frame is
numbered, so one task-local re-entered around each spawn carries the id through
Rust, Lua and Bun — no per-runtime invention.

Two record corrections this pass: [[the-inline-agent]] went back to `open`
(it was marked done while carrying an open child), and
[[the-context-inspector-opens-from-the-chat-editor]] was opened and attached
under it — the `/context` inspector has no entry point at all in `builtin/ui`,
though `agent {op:"context"}` serves it.

Open for the user, not for a coordinator: `refs/heads/main` moved from `1e2f2e4`
to `33c175d`, the tip of this working branch, by raw ref update rather than
merge. It is a fast-forward and nothing is lost, but `main` now claims this
branch's in-flight work as released. Three of the four sessions have ruled
themselves out. Nobody is touching it until the user decides.

### sys-6d close of pass

Landed and closed, each verified on the trunk after the merge:
[[rolling-context-retains-decision-evidence]], [[tool-graph-engine-is-the-ranking-database]],
[[check-boxes-lint-as-observable-behaviours]], [[pty-owns-the-terminal-grid]],
[[cartridges-prove-themselves-at-registration]].

The `momo` → `zirkle` migration landed as `c7a0e0d`, 559 files in one commit, on the
user's instruction once they confirmed the staged rename was theirs. Everything
tracked and dirty was staged except three memos other sessions were mid-write on;
no untracked file was added; `cargo check --workspace --all-targets` passes on the
result. It had been staged-and-uncommitted for over an hour, during which it swept
one session's unrelated commit and held every lane's gates red through the shared
`builtin/memory` symlink, which already depended on `zirkle`.

Four lanes are finished and blocked on the gitignored `builtin/memory` repo, which
a fifth session is renaming `kern` → `memory`. Each is rebased onto the renamed
trunk with `just check` exit 0 and every other `just test` step green:

- [[a-turn-carries-one-id-through-host-lua-and-bun]] — done, six boxes; needs the
  `zirkle` dev-dependency in `builtin/memory`, and its `justfile` edit overlaps an
  uncommitted one on the trunk.
- [[debug-mode-opens-and-closes-from-the-shell]] — 8 of 9 boxes.
- [[context-is-living-not-per-session]] — 5 of 6 boxes.
- [[the-palette-and-exit]] — 5 of 6 boxes.

Three small fixes in that repo unblock all four: quote the `description:` of
`research/the-watcher-stores-an-empty-body-as-a-document.md` (an unquoted
`text: ""` makes it invalid YAML), add `zirkle` to `[dev-dependencies]`, and make
`just memory-build` build the `memory_cartridge` bin its manifest now names.

Disk is a first-class constraint on this tree, not an incident: the volume hit
100% once and 99% twice in one pass, `target/debug` is 29G and each isolated gate
directory 9-11G. Corrupt rmeta and vanishing object files were the symptoms, and
they read as compile errors. The sessions agreed to sweep only their own gate
directories and none younger than four hours. [[lanes-do-not-poison-each-others-builds]]
owns the fix.

Five sessions shared this checkout. What kept it workable was asking who owned a
file rather than inferring it from an mtime, and landing a tree-wide change as one
commit instead of leaving it staged.

### sys-de analyst pass (closed)

Sys-de dispatched a spec band (analysts on unowned open memos) that depends on
[[the-ui-paints-the-grid]]. Six returns landed as `5c2ee7d`, all claims struck
(memos back to `open`, owner lines removed):

- [[the-embedded-terminal-is-gone]] — SPECCED (`builtin/pty/main.rs` drops
  Ring/read/print, CPR probe in `builtin/pty/tests/process.rs`); lane commits probe it.
- [[copy-mode-interacts-with-the-text]] — SPECCED (pty `region` op + grid test,
  painter copy overlay/test, controller wiring); probe green in-lane.
- [[the-sidebar-slides-over-the-shell]] — SPECCED; spec ported from its lane
  (`.momo` path) into the trunk memo at strike time.
- [[fresh-checkouts-can-run-the-gates]] — SPECCED; `just memory-bootstrap` +
  `memory-present` guard + note `memory-bootstrap.md`.
- [[tool-dispatch-and-routines-are-graph-nodes]] — analyst verified behavior,
  preempted by the coordinator's own work; no spec amendment.
- [[the-terminal-is-drawn-from-pty]] — SPLIT processed; subwork box ticked.

Outstanding when sys-de closed:

- Three analyst returns still in flight on their lanes (`pty-encodes-input`,
  `agents-query-the-tool-graph`, `the-agent-can-extend-and-verify-a-cartridge`).
  Whoever continues the spec band strikes their claims and commits/ports their
  specs by exact path only; nothing sweeps the index or the shared
  `builtin/memory` symlink, and no stash is popped in that repo (apply-by-SHA).
- `justfile` carries an uncommitted `--bin memory_cartridge` change (sys-52's
  integration); it is not anyone else's to commit.
- sys-52 ↔ sys-fb: whether to apply the quarantined zirkle cartridge port from
  the shared memory repo's `stash@{0}`/`stash@{1}` under the new naming is
  unresolved and left to them.

Post-close advisory from sys-6d: with `builtin/memory`'s record now merged into
the composed board, the open count went 29 → 71 and the axis walk from
`the-vision` no longer reaches most of them — the memory cartridge's memos hang
off their own root, and several memos changed parents in `5c2ee7d`. The next
pass's first step is a re-attach (repair the axis walk / re-base the walk), not
a dispatch. sys-6d is not adopting the three analyst orphans tonight; they stay
`active` with `analyst-*` owners as committed-probes-without-a-session markers.
`agents-query-the-tool-graph` (ad7cb11a) was killed at close: it looped on a
dead tmux-mcp `execute-command` (1800s idle timeouts). Its lane
`work/agents-query-the-tool-graph` holds an UNCOMMITTED probe — new
`builtin/toolgraph/` cartridge (main.rs, service.rs, cartridge.json,
init.lua, tests/) plus edits to `.momo/default/init.lua`, `Cargo.lock` and
`builtin/toolgraph/Cargo.toml` — mid E0599 inventory fix in `service.rs`.
Preserve the working tree before any lane sweep; the last commits there are
`d0ab19e` (struck claim) on `1e8d0a4` (compaction-eval corpus).
