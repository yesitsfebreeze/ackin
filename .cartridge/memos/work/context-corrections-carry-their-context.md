---
kind: work
description: "A correction made from any tool records the context it was made in, and the agent honours it from then on"
status: done
level: 10
priority: P1
estimate: 2h

---

# context-corrections

## Do

Corrections are context-based. Seeing something unwanted anywhere, the user
notes it down (composer, copy mode, palette — whatever surface is at hand) and
the correction is recorded together with the context it was made in: which
program is foreground (nvim is only an example), what files and buffers are
open, which command and cwd the pty last ran, the session. The agent already
has this state through `sessions` (buffers, files touched, agent state) and
`pty` (`commands`, `environment`).

From then on the agent knows: this is not wanted, or this is how things are
decided. The correction is durable knowledge — written to the memory bank and,
when it is a rule or routine, to the memos — with its context anchor as
provenance, and it is recalled when that context recurs, not just when the
exact words recur.

User request, 2026-09-12: corrections done everywhere — "Even if I'm in nvim
and I go into that Ctrl+G mode, I can correct things in nvim and we have the
context to know where and how to edit things ... It's about how every tool,
every context decision can be recorded since we know what we had open and what
state we were in."

## Spec

The probe (committed on the lane branch `work/context-corrections-carry-their-context`,
commit c47e304) built the whole vertical slice and it works.

**Files it touches**

- `builtin/harness/main.rs` — new `anchor` frame value: `resolve_shell` makes one
  pty `commands` scan that feeds both `terminal` and the anchor;
  `describe_anchor` renders "Session <id> (<name>) in <cwd>; shell foreground:
  `<cmd>` (running|exit N, in <cwd>); files touched this session: <files>" from
  the session (files) and the pty scan (running command, else newest finished);
  unit test `anchor_names_session_foreground_and_files_without_the_user_naming_them`.
- `builtin/harness/inspection.rs`, `inspection_tests.rs`, `working_tests.rs` —
  anchor plumbed through `Request`/`Frame` and the inspection variables.
- `builtin/memo/src/record.rs` — SEEDS entry for `system/corrections.md`.
- `builtin/memo/seeds/system/corrections.md` (new) — the recording convention:
  record a correction in the same turn through the memo tool (`kind: note` fact,
  `kind: routine` rule), quote the anchor line verbatim as provenance, put the
  anchor's distinguishing words (program, cwd, files) into the memo's `uses:`
  `when:` list, and include those words in `resolve` queries when the context
  recurs; the anchor governs where the correction applies.
- `builtin/memo/seeds/system/workspace.md` — composes
  "Context anchor (quote it when recording a correction): `anchor`".
- `builtin/memo/seeds/type/system.md` — documents the `anchor` placeholder.
- `builtin/memo/tests/tests.rs` — seed count 21 -> 22 and the composed-template
  expectations include the corrections memo.
- `.zirkle/memos/system/corrections.md` (new), `.zirkle/memos/system/workspace.md`,
  `.zirkle/memos/type/system.md` — the same three record edits saved through memo
  writes into the workspace record.

**What the probe already did**

- The anchor renders live: a real `zirkle run harness` context call on a probe
  session printed "Context anchor (quote it when recording a correction):
  Session <id> (probe) in <cwd>; files touched this session: src/lib.rs" inside
  the system-reminder frame (foreground was empty on a disposable PTY with no
  commands; the unit test covers the running-command and newest-finished paths).
- The record composes the anchor placeholder and the corrections memo (`zirkle
  run memo op system` verified).
- Recall demonstrated: a correction note whose `when:` carried the anchor words
  resolved first on an anchored `resolve` query and was absent from an
  unanchored query (the demo note was removed after the probe).

**What is left**: run the gates and the Check. Durable storage is the memos
with the anchor as provenance — the Check asks for "memory bank or memos", and
the host memory bank clears with the host, so memos are the durable leg and no
bank write path is added. Gates race the shared target and the shared
`target/debug` is at disk capacity; run cargo under
`CARGO_TARGET_DIR=/Users/feb/dev/sys/target/context-corrections-gate`.

**Steps**

1. `cd /Users/feb/dev/sys/.claude/worktrees/context-corrections-carry-their-context`
   (lane exists; the probe is commit c47e304).
2. `CARGO_TARGET_DIR=/Users/feb/dev/sys/target/context-corrections-gate cargo nextest run -p harness -p memo_cartridge`
3. `CARGO_TARGET_DIR=/Users/feb/dev/sys/target/context-corrections-gate cargo clippy -p harness -p memo_cartridge -p zirkle --all-targets -- -D warnings`
4. Live profile proof: `CARGO_TARGET_DIR=/Users/feb/dev/sys/target/context-corrections-gate cargo build -p sessions -p pty -p router -p agent -p harness -p memo_cartridge --bins`,
   then through `/Users/feb/dev/sys/target/context-corrections-gate/debug/zirkle`
   (cartridge binaries are found beside the running zirkle): `zirkle run sessions`
   create + `touch`, `zirkle run sessions` checkpoint to give the session a
   transcript, `zirkle run harness` context, and confirm the anchor line carries
   session, cwd and touched files the user never named.
5. Record and recall: through `zirkle run memo` with the workspace cwd, write a
   throwaway correction note (anchor quoted in the body, anchor words in
   `when:`), resolve with the anchor words as query (hit), resolve without them
   (absent), then delete the note file.

## Check

- [x] `cargo nextest run -p harness` passes
      `anchor_names_session_foreground_and_files_without_the_user_naming_them`:
      the anchor names session, foreground program and touched files with the
      user naming none of them.
      (`PASS [ 0.006s] (1/1) harness::bin/harness
      tests::anchor_names_session_foreground_and_files_without_the_user_naming_them`;
      full `cargo nextest run -p harness -p memo_cartridge`: 61 tests run:
      61 passed)
- [x] A live default-profile `zirkle run harness` context composes the anchor
      line (session, cwd, files touched) into the system prompt from the record.
      (fresh session + `touch check.rs` + checkpoint, then `zirkle run harness
      op context` on the gate binaries printed inside the system-reminder frame:
      "Context anchor (quote it when recording a correction): Session
      18d47dd1d16952b8-1 (check) in
      .../.claude/worktrees/context-corrections-carry-their-context; files
      touched this session: check.rs" — and the `system/corrections.md` body
      ("Recall is anchored too: ...") composed beneath it)
- [x] The seeded record composes `system/corrections.md` and the anchor
      placeholder; the bootstrap seed count is 22.
      (`PASS [ 0.728s] memo_cartridge service::tests::
      bootstrap_is_self_describing_and_leaves_other_zirkle_state_alone`
      asserting `assert_eq!(out["memos"], 22)`; `zirkle run memo op system` on
      the lane record lists corrections among the system memos and its
      template carries "Context anchor (quote it when recording a
      correction): `anchor`")
- [x] A correction recorded as a memo with anchor provenance resolves on a
      query carrying the anchor words and does not surface on a query that
      does not carry them.
      (throwaway `note/context-corrections-demo.md` written through the memo
      tool — anchor quoted verbatim in the body, anchor words `nvim`,
      `check.rs` in `when:` — `resolve` usage edit: query "nvim check.rs
      formatting" ranked it first, `why: matched ... when: check, nvim, rs`,
      score 11.5; query "keep code style as written" without the anchor words
      returned `total: 0`. Demo note deleted after the proof.)

```sh
cd /Users/feb/dev/sys/.claude/worktrees/context-corrections-carry-their-context
export CARGO_TARGET_DIR=/Users/feb/dev/sys/target/context-corrections-gate
cargo nextest run -p harness -p memo_cartridge
M="$CARGO_TARGET_DIR/debug"; R="$PWD"
# the record composes the anchor placeholder and the corrections memo
"$M/zirkle" run memo "{\"cwd\":\"$R\",\"op\":\"system\"}" | grep -c "Context anchor"
# a live harness context renders the anchor from session state alone
ID=$("$M/zirkle" run sessions "{\"cwd\":\"$R\",\"op\":\"create\",\"name\":\"check\"}" | python3 -c 'import json,sys;print(json.load(sys.stdin)["id"])')
"$M/zirkle" run sessions "{\"cwd\":\"$R\",\"op\":\"touch\",\"id\":\"$ID\",\"file\":\"check.rs\"}" >/dev/null
# a session needs its transcript before the harness serves it
"$M/zirkle" run sessions "{\"cwd\":\"$R\",\"op\":\"checkpoint\",\"id\":\"$ID\",\"expected_revision\":0,\"records\":[{\"v\":1,\"kind\":\"message\",\"run\":\"r1\",\"message\":{\"role\":\"user\",\"content\":\"hi\"}}],\"agent\":{\"phase\":\"completed\"}}" >/dev/null
"$M/zirkle" run harness "{\"cwd\":\"$R\",\"op\":\"context\",\"session\":\"$ID\"}" | grep "Context anchor"
```

## Actual

All four Check boxes are `[x]` and the Check `sh` block passed end to end on the
lane worktree `.claude/worktrees/context-corrections-carry-their-context`
(branch `work/context-corrections-carry-their-context`, rebased onto the trunk
tip): `cargo nextest run -p harness -p memo_cartridge` 61 passed including
`anchor_names_session_foreground_and_files_without_the_user_naming_them`;
clippy `-p harness -p memo_cartridge -p zirkle --all-targets -D warnings` clean;
a live `zirkle run harness op context` on the gate-built binaries rendered
"Context anchor (quote it when recording a correction): Session
18d47dfb899f3810-1 (check) in .../context-corrections-carry-their-context;
files touched this session: check.rs" with the `system/corrections.md` body
composed beneath it; bootstrap seed count asserted 22; a throwaway correction
note with the anchor quoted as provenance and `nvim`, `check.rs` in `when:`
ranked first on an anchored `resolve` (score 11.5, `when: check, nvim, rs`)
and a query without the anchor words returned `total: 0` (note deleted after
the proof). The trunk copy of this memo carries the same ticks; both the trunk
record edits and the lane's record edits are preserved in the rebase. Not
landed and the worktree kept, left to the coordinator.

The full gates on the rebased lane (isolated `CARGO_TARGET_DIR=.../context-corrections-gate`): `just check` clean (fmt, workspace clippy `-D warnings`, bun ui check), `just test` green after two isolated reruns — the first run's two `zirkle` failures (`inventory_distinguishes_provider_states...`, `wrapped_process_isolation_metadata_and_dependency_restart_compose`, one panicking `event: Elapsed(())`) passed alone immediately, i.e. contention timeouts from parallel lane gates, and `test_router.py` needed the `proxy` binary built into the private target first (`cargo build -p proxy`), matching the memo note on foreign/gate binaries — then `just all` (check + test again) fully green, exit 0.
