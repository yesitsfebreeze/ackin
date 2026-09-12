---
kind: note
description: "Current work-pass ownership, evidence, and next actions"
uses:
  - usage: "[[read-usage]]"
    when: ["Resuming the work pass or collecting task results"]
    tags: [documentation]
---

# work-checkpoint

## codex-work-2026-09-09

User requested finishing all tasks. Initial kern plan: 18 open, 1 ready, 17 later, 0 asking, 0 claimed, 0 off-axis. [[process-plugins-register-through-lua-rpc-roundtrip]] landed as `3d539a7` and `4ed0e08`; its lane closed. Focused process tests 5/5 and combined `just all` 23/23 plus fmt/Clippy/doc tests passed.

Lane workflow bootstrap landed as `0130164`; six disposable-repository safety tests passed. Its worktree and branch were closed after ownership release and ancestry/cleanliness checks. Coordinator owns record paths. [[local-work-lanes]] records this checkout's integration and record layout. The pre-existing `.agents/` and `agent-ad54408b866f8bfce` worktree are not owned by this session.

Lua registration landed through `c09bedb`, all five acceptance checks complete; `just all` 28/28 plus fmt/Clippy/docs passed; lane closed. 4/18 task scopes done; atomic session storage landed as `1db861d`, lane closed; full gate 39/39 plus fmt/Clippy/docs passed; policy landed and its lane closed. Combined `just all` 32/32, fmt/Clippy/docs passed. Active next wave: `fs_ops` owns `.claude/worktrees/fs-operations`, `lane_workflow` owns `.claude/worktrees/session-checkpoint`, and `policy_worker` owns `.claude/worktrees/core-boundary`. All use isolated Cargo targets and the shared conventions in [[coding-plugin-contracts]]. `lua-only-core-boundary` is claimed and dispatched.

Next: collect each completed lane serially and dispatch newly ready tasks. User directs finder.nvim-style composable agent search; composable-agent-search now governs the search memo, which requires a worker probe/re-spec before completion. Fs mutation work continues unchanged. User asked about RPC fixtures and kern transport: fixture source moved under tests; kern transport JSONL codec is potentially reusable but generated typed RPC envelopes differ, so no protocol migration made.


## sys-eb 2026-09-09 19:40

Re-dispatch pass. Previous codex-work session died mid-flight: its 3 implementers (fs-operations, core-boundary, session-checkpoint) were dead, lanes held partial work, 0 boxes ticked. Re-dispatched all 3 into their existing lanes. session-transcript-durability-run-checkpoint returned DONE (6/6 boxes, 16 tests, just all green) — landed on trunk as 045e023/4909616/18d802f, lane removed. harness-plugin became ready after that landing; claimed (sys-eb) and dispatched into a fresh lane. In flight: fs-operations, core-boundary, harness-plugin. Next: collect returns, land serially.


## sys-eb 2026-09-09 20:06

fs-tool-plugin-file-operations returned DONE (7/7 boxes, 15 fs tests, just all green) — landed on trunk as 02fd99e/367383a/890dc9c, lane removed. fs-tool-plugin-file-search became ready; claimed (sys-eb) and dispatched into a fresh lane. In flight: lua-only-core-boundary, harness-plugin, fs-tool-plugin-file-search. Next: collect returns, land serially.


## sys-eb 2026-09-09 20:51

harness-plugin returned DONE (6/6 boxes, 10 unit + 1 integration test, just all green after one flaky core test re-run) — landed on trunk as 4519e93/e645213, lane removed. safe-transcript-compaction became ready; claimed (sys-eb) and dispatched into a fresh lane. In flight: lua-only-core-boundary, fs-tool-plugin-file-search, safe-transcript-compaction. Next: collect returns, land serially.


## sys-eb 2026-09-09 22:12

safe-transcript-compaction returned DONE (5/5 boxes, 19 harness tests) — landed as 10e279c, lane removed. lua-only-core-boundary's implementer worked (2 commits, clean lane) but never returned; verified its Check directly (32 core tests + sh block green), closed memo, landed as 6615fed, lane removed (daemon side-effect deletions stashed). fs-tool-plugin-file-search: two implementer dispatches died (transcript corruption; agent looped at same probe point) — failure recorded as memo's first question, re-claimed and re-dispatched with hardened brief. Board 9/18, 1 in flight (fs-search). Next: collect fs-search, dispatch ready memos.


## sys-eb 2026-09-09 23:20

fs-tool-plugin-file-search: three implementer dispatches died infrastructurally (transcript corruption, agent loop, stream watchdog), exhausted the one-redispatch rule, so I implemented it sequentially on the coordinating thread per the routine. Composable typed search landed: files/dirs/grep/matches/files_of stages, ref refinement, glob/grep convenience over the same engine, --no-config isolation, deadline+cancel reap. 96/96 workspace tests, clippy clean. Landed as 45acf8f/60c6e3d + memo close 61b23ea, pgrep-race fix 1239402; lane removed. Board 10/18, 0 in flight, 0 ready.


## sys-eb 2026-09-10 01:30

shell-tool-plugin returned DONE — landed 28bdb8f (feat 503228f + docs 7768ea6), lane removed. tool.bash bounded with process-group termination, deadline/cancel/disposal reap, truncation markers, 111 tests green standalone (full-run flake is the known shared-target race). memo close committed on trunk. In flight: agent-run-control, memo-tool-plugin. Board 11/18.


## sys-eb 2026-09-10 01:45

memo-tool-plugin returned DONE — landed 6c0f365/52b2aaf/bbdccc6, lane removed. Bounded kern MCP adapter forwarding to kern's own memo op; 14 tests green, clippy clean. memo close committed on trunk (status+actual via kern write). Board 12/18. In flight: agent-run-control. Later: agent-loop-plugin, streaming-agent-output, terminal-agent-client, coding-profile-integration, coding-agent-vision.


## sys-eb 2026-09-10 02:30

agent-run-control: implementer completed work (3 commits, 8 run_state tests, clippy clean) but never returned the formal verdict — verified its Check directly and closed memo. Landed 5681da7/d9caa34/98eb1a5 (Cargo.toml conflict with shell/memo resolved keeping all members), lane removed. agent-loop-plugin became ready; claimed (sys-eb) and dispatched. Board 13/18. In flight: agent-loop-plugin. Later: streaming-agent-output, terminal-agent-client, coding-profile-integration, coding-agent-vision.


## sys-eb 2026-09-10 04:45

agent-loop-plugin landed (970f98b/74a5275/51c854e/9f45fa7/7ea60ea): non-streaming model driver, durable Chat tool-group closing, recovery, real process-to-host RPC test loop. 18 agent tests green, memo closed on trunk, lane removed. Board 14/18. streaming-agent-output + terminal-agent-client became ready; both claimed (sys-eb) and dispatched in parallel. Later: coding-profile-integration, coding-agent-vision.


## sys-eb 2026-09-10 05:58

terminal-agent-client landed (4b400ff/2380cbd): TUI agent socket client with multiplexed replies/events/approvals/cancel, pre-start buffering, sequence-gap recovery, streaming deltas, disconnect uncertainty. 8 tui tests green, clippy clean. Memo closed on trunk (coordinator). Lane removed. Board 15/18. In flight: streaming-agent-output (2 commits: SSE parser + stream driver with router lease + one-shot pre-visible retry). Later: coding-profile-integration, coding-agent-vision.


## sys-eb 2026-09-10 06:22

streaming-agent-output landed (711a795/0c8221b/662b7ce/360a03a): leased router sessions, bounded SSE parser, one-shot pre-visible retry, stream_error fail, exactly-once release. 33 agent tests green, memo closed on trunk, lane removed. Board 16/18. coding-profile-integration (final composition terminal) claimed + dispatched. Later: coding-agent-vision (close-out).


## sys-eb 2026-09-10 08:40 — CLOSED

coding-agent-vision 18/18, 100%. Final verification pass closed it: fixed a real harness bug (injectable clock never injected — resolve_date silently fell back to system time; the hardcoded-2026-09-09 test rotted at rollover) and its profile-smoke pinned expectation. just agent-smoke 7/7, just check green, just test 168/168. All 18 memos status: done, all lanes landed and removed. Loop complete.


## sys-claude 2026-09-12

Board finished: 43 work memos, 42 done, 1 open — [[the-vision]] itself, which stays
open as the record's axis with every box green. Zero open memos off the axis.

Written this session: the vision and its six children, plus
[[spec-a-work-memo]] and [[implement-a-work-memo]] as `kind: routine` memos, so
the two procedures live in the record rather than in any one agent's
configuration.

Delivered: the whole memory-resolver chain under `builtin/memo` (eight memos —
empty resolves say so, items carry kind, status, provenance and freshness,
supersession is ordered and called out, and the loop is proven end to end
through the model-facing surface); `core/tests/test_shell.py` and
`core/tests/test_router.py`, which are new and carry the wrapped shell, the
inline agent, and the router's ranking, recovery, health, credential
containment, launch and concurrency; and the lane workflow itself, which could
not run its gates at all when this session started.

Gates: `just check` and `just test` green on the trunk, three consecutive clean
runs. No lanes open. `main` is at the last landed commit.

Worth knowing for the next pass, all recorded in the memos that found them: the
memo tool's `description` is prompt tail and a long one fails the compaction
test; python terminal tests must each get their own process, which `just test`
now arranges; the fake providers in `test_router.py` run as processes for the
same reason; and a screen buffer that is never cleared will satisfy almost any
assertion you make against it.

Next: nothing is ready. New work attaches under whichever child of
[[the-vision]] contains it, or becomes a seventh.


## claude-sys 2026-09-12 07:13

Pass on the runtime-audit wave. `kern plan` is not available in this
workspace; the board was derived through the memo tool (list/read per work
memo) with the routine's band rules applied by hand. Scan: 0 off-axis, 0
asking, 2 claimed (codex-work-2026-09-12 owns rpc-contracts and
terminal-profile-starts-only-needed-services), 1 ready,
3 in spec.

Found an uncommitted trunk edit on
[[the-agent-can-discover-its-own-program]]: a codex analyst had written its
`## Spec` (probe on `work/program-map`, `.claude/worktrees/program-map`, 4h
left) and dropped its claim without committing. Committed it as `edfd59c` so
lanes see the Spec, wrote this session's claims, and dispatched: one
implementer on the-agent-can-discover-its-own-program (continues the
program-map lane), analysts on context-date-needs-no-clock-cartridge,
rolling-context-retains-decision-evidence and pty-owns-the-terminal-grid.
Blocked behind claims or needs: fresh-checkouts, ci-proves, debug-mode,
extend-and-verify, the whole pty-grid chain (pty-encodes, ui-paints,
embedded-terminal-gone, gutter pair, sidebar, sub-agents).

Next: collect the four returns serially, land lanes, re-scan for unblocked
memos.


## claude-sys 2026-09-12 (continued)

Collected: program-map landed `42785e8` (gate green; router failures were a
missing proxy binary in the isolated target, red then green on the same
binary); context-date landed `9bad9a5`, lane closed; pty-owns SPECCED from its
probe `513a340` after its session was pinned to a deleted worktree and its
hook failed closed (root cause: my lane-rm removed the worktree under it —
land order must wait for session teardown); living-context SPECCED with a
surfaced gap, now the memo [[the-memory-bank-is-wired-into-the-surface]];
tool-graph SPLIT into four children, parent claim struck, children held in
spec for the next wave (seven concurrent gates caused SIGKILL/OOM earlier).
In flight: palette analyst, rolling-context and context-corrections
implementers. Queued in spec: memory-wiring, check-boxes lint, the four
tool-graph children.

Worktree note: `.claude/worktrees/program-map` still exists because lane-rm
refused its runtime artefacts (.zirkle/credentials, .zirkle/router, target/) —
needs a human call to delete. Disk hygiene: deleted 12k stale deps artifacts
(>1 day old); shared target/debug still bloats under concurrent lane builds
(62G observed) — cargo never GCs; a coordinated cargo clean is the real fix.

Direction recorded from the user: invisible agent with /btw-style dispatch
([[the-palette-and-exit]]), copy mode with wiki links and corrections
([[copy-mode-interacts-with-the-text]]), context corrections anchored to any
tool ([[context-corrections-carry-their-context]]), living context across
agents ([[context-is-living-not-per-session]]), usage-ranked tool graph
([[a-usage-ranked-tool-graph-serves-the-best-way]]), desktop/browser as a
later client (decision memo). Unlazy deltas integrated: evidence-void rule in
[[implement-a-work-memo]], [[land-a-worked-lane]] routine, check-box lint as
[[check-boxes-lint-as-observable-behaviours]].

Next: collect the three in-flight returns, land serially, then dispatch the
spec-band queue (memory wiring, lint, four tool-graph children) in bounded
waves.

## claude-sys 2026-09-12 (close)

Session closing (user request). Handoff to sys-52: three finished lanes need
rename-rebase + full gates + land per [[land-a-worked-lane]] —
[[the-resolver-runs-on-the-tool-graph-engine]] (lane tip b452bb7, rebased onto
c7a0e0d, rename-reconciled, just check green, all crates green; only the
memory-repo leg of just test blocked on the shared tree settling),
[[tool-dispatch-and-routines-are-graph-nodes]] (all four boxes ticked at
5fc14dd, needs rebase onto c7a0e0d + gates),
[[the-memory-bank-is-wired-into-the-surface]] (Done at bf3506f pre-rename,
needs rename rebase + gates; its worktree owns the real builtin/memory dir).
Two quarantine stashes in the shared builtin/memory repo (stash@{0}, stash@{1},
labelled sys-fb) hold the user's uncommitted zirkle memory-cartridge port —
sys-52 applies them by SHA; pin lifts on application. Resolver lane symlink
restored to the shared repo; /tmp/sysfb-memory-gate-clone disposable. Peer
sessions sys-de closed (5c2ee7d); sys-6d active. Landing rules in force:
single-path staging, full just check + just test before any land, isolated
CARGO_TARGET_DIR per lane, gate dirs owned by their creating session.

## sys-38 2026-09-12 (work pass)

`kern plan` still absent (no kern MCP; the installed kern binary is the memory
DB); board derived through the memo tool as before. Scan: 0 off-axis, 1 asking (question/streamline-the-overloaded-names still open), 3 ready dispatched, 3 spec
dispatched, 12 still queued in spec.

Correction: the attempted answer to question/streamline-the-overloaded-names
came from a tool result that was later identified as non-human input, not a
real user decision. The question is restored open; the false decision
(both-overloaded-names-are-streamlined) and its two rename work memos were
removed. The only durable user correction from this later exchange is the
justdown-shaped tool-contract direction, recorded as
decision/the-tool-contract-is-a-memo and work/a-tool-is-declared-by-its-memo.

Attached (were off-axis): headless-policy-approval-channel (under
zirkles-tools-serve-any-agent), long-horizon-recall-benchmark (under
the-record-feeds-the-agent), per-cartridge-versioned-test-runner (under
the-work-can-be-proven); all three got level 10.

Dispatched in one wave, owners written as sys-38: implementers on
a-cartridge-declares-what-it-needs, a-nested-cartridge-is-a-first-class-entry,
the-board-is-channels-of-lines (all carried a spec from earlier analysts);
analysts on lanes-do-not-poison-each-others-builds, every-tool-is-one-command,
headless-policy-approval-channel. Lanes use isolated CARGO_TARGET_DIR
(/tmp/zirkle-targets/<leaf>); agents were told not to land — the coordinator
lands serially.

Orphaned claims, NOT touched this pass: the sys-work/sys-opus/codex-work
2026-09-12 owners (pty-encodes-input, the-sidebar, terminal-drawn-from-pty,
agents-query-the-tool-graph, fresh-checkouts, extend-and-verify, rpc-contracts,
terminal-profile; blocked: context-is-living, debug-mode-opens, the-palette —
each blocked only on a lane gate run, and their work looks landed by sys-52).
cartridge-31 confirmed it owns none. A later pass can verify those last boxes
on trunk and close them.

Spec queue for the next wave: sub-agent-sessions-record-parent-and-mailbox,
the-context-inspector-opens-from-the-chat-editor, the-reload-test-is-not-flaky,
a-search-ranks-current-guidance-over-delivered-history,
an-event-declares-its-type, one-search-covers-the-record-and-memory,
the-memory-workspace-is-tracked-not-patched,
per-cartridge-versioned-test-runner, a-tool-is-declared-by-its-memo (check .pearde/prds overlap first — pearde
has a verify-one-cartridge PRD), long-horizon-recall-benchmark.

Trunk: main pushed and in sync with origin (the handover blocker resolved).
Uncommitted trunk edits (builtin/memo README+record+tests, core/main.rs,
core/tests/profile.rs, core/turn.rs, .cile/, two note memos) belong to a dead
session, not cartridge-31; left untouched.

Defect found: the memo tool refuses writes to note/work-checkpoint.md with
"duplicate memo leaf" because the memory cartridge ships
dashboards/work-checkpoint.md; type.md says a workspace memo shadows a shipped
one, so this write should pass. Recorded as
work/a-shadowing-write-passes-the-leaf-check. This checkpoint section was
appended directly for that reason.

Next: collect the six returns serially, land clean lanes with `just land`,
re-scan.

