---
kind: work
description: "Two gutter columns, one icon per command block per side, sticky at the top row through scrollback"
status: open
estimate: 2d
needs:
  - "[[the-ui-paints-the-grid]]"
---

# the-gutter-shows-both-sides

## Outcome

Two columns left of the grid, present in every mode. The right column is the
shell's: running, exited zero, exited nonzero, or a program holding the
alternate screen, dim when the user typed the command and bright when the agent
ran it. The left column is the agent's: working on this block, waiting for an
approval, failed here, or carrying a reply the user has not read. A block with
nothing to say on a side leaves that cell blank.

An icon sits on the first visible row of its block. When the block's start has
scrolled above the viewport the icon stays on the top row until the block below
it reaches the top, so the reader always sees which block the rows in front of
them belong to. The same two columns run down the full transcript, mirrored:
each transcript line knows its shell block, and that block's shell icon sticks at
the top of the transcript the same way.

State is legible without colour: each icon is a distinct glyph, colour is
reinforcement.

## Check

- [ ] `cargo test -p pty blocks_report_delimit_commands_with_states` and `bun test tests/gutter.test.ts`: a failing `false` in a real shell comes out block state `failed` with a later start line than the command before it, and the shell column shows the `!`/`✕` icon on that block's first visible row.
- [ ] `bun test tests/gutter.test.ts`: moving one block's agent state working → waiting → reply reads `●` → `?` → `✉` in the left column on the same row.
- [ ] `bun test tests/gutter.test.ts`: a viewport opened mid-block pins that block's icons on the top row; moving the top one block-line further up hands the top row to the block above; a block starting inside the viewport gets its icon on its own row while the old one stays pinned.
- [ ] `bun test tests/gutter.test.ts` asserts eight distinct glyphs and eight distinct ASCII fallbacks; once step 3 of the Spec lands, a captured frame with `NO_COLOR=1` still shows all four shell and four agent states as distinct characters in the gutter.

## Approach

The UI asks `pty` for the block id of each visible row and the command record
behind it; the agent side comes from the transcript, whose lines record the
block that was current when they were written. Sticky is a comparison between the
first visible row's block and the block starting above it, not a second data
structure. Glyph table in one place, with the ASCII fallback the design record
asks for.

Surface amendment, 2026-09-12: indicators coexist with the latest-tool footer,
main/dynamic status and Ctrl+G/Ctrl+F views under
[[the-agent-surface-preserves-the-visible-shell]]. They never replace those rows
or put agent text into the editor stream.

## Spec

The gutter is two columns left of the grid, present in every mode. The probe
(pass one, committed as `cba766f` on this lane) built both data sides and the
pure mapping; the implementer continues it. The row wiring is gated on the
parent [[the-ui-paints-the-grid]] — only its grid can say which block-line each
viewport row is.

Probe, done:

- `builtin/pty/blocks.rs` — ordered command blocks over the shadow screen's
  line space: `{id, command, start, end, state}` with state in
  running/ok/failed/alt, sampled at the OSC 133 marks; `lines`, `at(line)`,
  `blocks()` and the `blocks` op result `{lines, blocks, current}`.
- `builtin/pty/main.rs` — the `blocks` op and the BlockLines fed alongside
  the screen, resized and created in `open`.
- `builtin/pty/tests/process.rs` — `blocks_report_delimit_commands_with_states`
  drives a real shell through `tool.shell`: `echo hi` then `false`, asserts the
  failing command is `failed` and its start line is later than the `ok` one.
- `builtin/ui/ui/gutter.ts` — the pure sticky mapping: `blockAt(line)` owns a
  line (latest start at or before it), `rows(firstLine, height, blocks)`
  returns one icon per block on its first visible row, pinned to the top while
  the block's start has scrolled above the viewport. One glyph table with
  distinct glyphs and distinct ASCII fallbacks across all eight states
  ([[tui-accessibility]]: never colour alone); the shell cell dims when the
  user typed the command and is bright when the agent ran it.
- `builtin/ui/tests/gutter.test.ts` — the eight-state table, the
  working → waiting → reply lifecycle, the sticky scroll comparison, blank
  cells, the dim/bright bit.

Remaining steps, in order:

1. `builtin/ui/ui/terminal.tsx` — wrap `shell.terminal`'s `<terminal>` in a
   row box with a 2-wide gutter column beside it; in the read pump also poll
   `term {op:"blocks"}` and, once [[the-ui-paints-the-grid]] has landed the
   viewport-to-block-line mapping, draw `gutter.rows` into the column with
   the dim bit from `agentRan`. Tool: terminal (semantic navigation); run
   `bun --conditions=browser test` after.
2. The transcript mirror — `chat.transcript` (same file) records each line's
   shell block — the `current` block id when the line was written — builds the
   transcript's own block runs, and renders the same two columns with the same
   `rows()` sticky. Tool: terminal.
3. A render-level check — a test that mounts `surface.default`, runs a failing
   command and a waiting approval, and captures the frame with `NO_COLOR=1`:
   eight distinct characters in the gutter. Tool: terminal.

`just all` from the trunk is the integration gate; the lane's pty tests need a
lane-local `CARGO_TARGET_DIR` because the shared target is cross-lane.
