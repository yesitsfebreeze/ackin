---
kind: work
description: "A copy mode freezes the pty grid so the cursor can move, copy, follow wiki links and send corrections to the record"
status: open
level: 10
priority: P1
estimate: 6h
needs: ["[[the-ui-paints-the-grid]]"]
---

# copy-mode-interacts-with-the-text

## Do

A mode where the live pty freezes and the grid becomes text the user can
interact with:

1. Freeze and move: a key enters copy mode; the terminal stops taking input
   for the shell, the viewport holds still, and the cursor moves over the
   grid (rows, scrollback included). The shell keeps running underneath.
2. Copy: select a region with the cursor, copy it to the clipboard; copy a
   whole session block (one command's prompt through output, via the marks
   rows) in one action.
3. Wiki links: paths and memo links visible on the grid can be followed —
   Enter (or a dedicated key) on a link opens the file (or the memo) in the
   user's editor.
4. Corrections: selected text can be sent to the record as a correction — a
   note or memory entry anchored to what was on screen — into the memory bank
   and the memos, without leaving the mode.

Selection content and target record writes go through the existing memo tool
and session services; nothing bypasses [[lua-only-core-boundary]].

## Spec

Files (all committed on the lane `work/copy-mode-interacts-with-the-text`):

- `builtin/pty/grid.rs` — `region(&self, from: u64, to: u64) -> String`
  extracts absolute rows `from..to` as plain text, trimming each and skipping
  aged-out rows, clamped to the live bottom; test
  `region_extracts_command_prompt_through_output`.
- `builtin/pty/main.rs` — the `pty` op `region` dispatches `{"from","to"}`
  (base64-free plain text, unlike `read`) to it.
- `builtin/ui/ui/painter.ts` — the frozen-grid overlay: `copyMode`,
  `copyCursor`, `copyAnchor`, `setCopyMode`, `setCopyCursor`, `copyText`
  (the rectangular selection from the stored rows), and the key/paste/mouse
  interception that routes everything to `onCopyKey` in the mode (F8 always);
  the selection paints inverse and the cursor a swapped-color block.
- `builtin/ui/ui/terminal.tsx` — the controller: F8 freezes the pump (frames
  stop applying while `copy.on`, the shell keeps running), arrows/vim keys and
  `space` move and anchor the cursor over absolute rows (scrollback included,
  auto-scrolling via the `pty` `scroll` op when the cursor leaves the
  viewport), `c`/`y` copy the selection or — with no anchor — the whole marks
  block (`commands` rows `row..next-1` through the `region` op) to the
  clipboard, `r` writes the selection back as `note/copy-mode-<stamp>.md`
  through the `memo` `write` op without leaving the mode, `enter` opens the
  path or `[[leaf]]` under the cursor in `$VISUAL || $EDITOR || nano` through
  the shell (`pty` `write`), `esc`/`q` exit and return the view to the live
  bottom.
- `builtin/ui/tests/painter.test.tsx` — the copy-mode behaviour test: F8
  interception, frozen key forwarding, `copyText` rectangular selection.

Steps (each checked by the command beside it, run from the repo root unless
the tool lives under `builtin/ui`):

1. `cd builtin/ui && bun test tests/painter.test.tsx` — the overlay paints and
   the mode intercepts keys (terminal). Probe pass: 6 pass.
2. `cd builtin/ui && bunx tsc --noEmit -p tsconfig.check.json` — the controller
   and painter typecheck (terminal). Probe pass: clean.
3. `cd builtin/ui && bun test` — the whole ui suite stays green (terminal).
   Probe pass: 26 pass.
4. `cargo test --manifest-path builtin/pty/Cargo.toml` — `region` extracts a
   command block (terminal). Probe pass: 10 lib + 7 process.
5. `just run` — the live hand: F8 freezes the shell (output keeps flowing once
   unfrozen), arrows page through scrollback and back with the view, `space`
   anchors a visible inverse selection, `c` puts exactly the selected cells on
   the clipboard, `b` the whole command block (prompt through output), `r`
   lands a `note/copy-mode-*.md` visible through the memo tool, `enter` on a
   path opens it in `$EDITOR` inside the shell (terminal).
6. Memory bank: add the `memory` cartridge to the default profile and route
   the correction's second target through `memory` `ingest` (the bank half of
   Do 4 is defined work: the cartridge is not loaded in the default profile
   today, so the probe lands the memo-note path and leaves the bank write to
   the implementer).

Probe (analyst, lane commits `6891799` `0a3e34c` `0ad22e4`): the `region` op
and its grid test, the painter copy overlay and its test, and the full
controller wiring are committed on the lane. `bun test` (26 pass), `bunx tsc`
(clean) and `cargo test --manifest-path builtin/pty/Cargo.toml` (17 pass) are
green in-lane. The lane snapshot predates the trunk's in-flight `.momo →
.zirkle` record/crate rename, so the lane's `just check` cannot build
`builtin/memory` (its sub-workspace migrated to `zirkle` while the lane's `core`
is still `momo`); the same `cargo check --manifest-path
builtin/memory/Cargo.toml` finishes on the current trunk checkout, confirming
the breakage is the stale lane, not this probe. Remaining defined work: step 5
(live-shell verification) and step 6 (memory bank).

## Check

- [ ] In copy mode the shell keeps running while the view is frozen, and the
      cursor reaches scrollback rows.
- [ ] A selection copies exactly the selected cells; a marks block copies the
      whole command block.
- [ ] Following a link on the grid opens the named file or memo.
- [ ] A selection sent as a correction lands in the record with its anchor,
      visible through the memo tool.

```sh
cd builtin/ui && bun test tests/painter.test.tsx        # frozen forwarding + copyText region
cd builtin/ui && bunx tsc --noEmit -p tsconfig.check.json   # controller + painter typecheck
cd builtin/ui && bun test                               # full ui suite
cargo test --manifest-path builtin/pty/Cargo.toml       # region op extracts the command block
```

User request, 2026-09-12: "go into a copy mode where we can move the cursor
around and freeze the current PTY and then we can follow vicky links to open
files. We can copy sessions. We can basically interact with the text. We can
also, in that mode, select text and send corrections to the kernel, the memory
bank, and the memos."
