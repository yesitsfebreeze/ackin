---
kind: work
description: "The UI draws the shell from pty's grid and forwards events, with no emulator of its own"
status: open
estimate: 2d
needs:
  - "[[pty-owns-the-terminal-grid]]"
  - "[[pty-encodes-input]]"
---

# the-ui-paints-the-grid

## Outcome

`shell.terminal` becomes a painter: it asks `pty` for the rows that changed
since the version it last drew, writes their cells into the OpenTUI buffer, and
places the cursor where the viewport says. It forwards key, mouse and paste
events without encoding them, and forwards its own size so the emulator and the
PTY resize together. Nothing in the UI parses escape sequences.

Colours come from the grid as they are: a cell carrying the shell's default
foreground or background is painted in the host's, which is what `carryTheme`
currently reconstructs by inspecting pixels after the fact. Scrolling the shell
moves `pty`'s viewport, so mouse wheel and any scrollback key work on the same
offset the gutter aligns to.

Drawing keeps up with a program that redraws continuously: a full-screen editor
scrolling a file, and a command printing fast, both stay smooth at the sizes a
real terminal runs.

## Spec

`shell.terminal` is a painter, `GridShellRenderable` in
`builtin/ui/ui/painter.ts`. It owns no emulator state: `pty` keeps the grid and
ships only the rows that changed since the last drawn version.

Files:

- `builtin/ui/ui/painter.ts` — new. `GridShellRenderable extends Renderable`;
  `apply(rows, view, version)` maps absolute `y` to the screen (`screenY =
  y - total + rows + offset`) and stores the rows; `renderSelf` paints the
  whole stored grid into the buffer with `buffer.setCell(x, screenY, char,
  parseColor(foreground(fg)), parseColor(background(bg)), styleFlags(style))`
  and places the cursor from the viewport (1-based OpenTUI convention). All
  overlaid layers (`renderBefore={clearOverlay}`) clear buffer cells, so a
  full repaint every render pass is required, not the old pixel post-compose.
  Cells carry `fg`/`bg` as `"default"` | 0–255 index | `"#rrggbb"` and style
  letters `b d t u v h s`; `default` is repainted with the host palette at
  paint time (`usePalette(slots, fg, bg)`). Key, mouse and paste events are
  forwarded unencoded via `onEvent` for `pty` to encode (no escape parsing in
  the UI). `screen()` returns the stored grid as text for tools and tests.
- `builtin/ui/src/term.ts` — `term` service dispatches `write`, `resize`,
  `print`, `frame`, `viewport`, `event` to `pty`, keeps `read` for replay, and
  gains `status` (`{ exit, last }`) so the pump can stop on shell exit.
- `builtin/ui/ui/terminal.tsx` — swap the embedded terminal for
  `<gridShell ref onEvent onTerminalResize>`, register it with
  `extend({ gridShell: GridShellRenderable })`, and replace the byte outbox
  with a pump: poll `frame(after)`/`viewport`/`status`, call
  `term.apply(rows, view, version)`, and apply the detected host palette to
  the painter (`usePalette`) instead of a post-compose pass. `carryTheme` and
  the old pixel-inspection code are deleted.
- `builtin/ui/tests/painter.test.tsx` — new; paints rows and cursor, applies
  the host palette to `default`-marked cells, keeps stored rows across an
  unchanged version, sustains a full-screen redraw, and forwards keys/paste
  as unencoded events.
- `builtin/ui/tests/transcript.test.tsx` — the surface interplay test drives
  the painter with `apply` (its `term` op mock blocks `frame` so the pump
  stays idle), proves Ctrl+F stays with nvim in shell mode by forwarding the
  key as an `event`, and asserts the grid redraws after the transcript overlay
  closes.

Steps:

1. Land the two `needs`: `[[pty-owns-the-terminal-grid]]` (the emulator
   serving `frame`/`viewport`) and `[[pty-encodes-input]]` (key encoding) —
   their lanes are in flight; the UI keeps the service shape, so the swap is
   one commit on top of them.
2. `bun install --cwd builtin/ui --frozen-lockfile`, then
   `bun run --cwd builtin/ui check` and `bun run --cwd builtin/ui test`
   (25 tests, painter + transcript included).
3. Run the real `nvim` passage from `core/tests/test_shell.py` against the
   painter, cursor landing where nvim put it.
4. Run the measurement (below) before and after any transport change, and
   record both rows/s in this memo's `## Result`.

## Check

- [ ] `bun run --cwd builtin/ui test` passes all suites including
      `tests/painter.test.tsx` (rows and cursor land where the viewport says)
      and `tests/transcript.test.tsx` (the grid survives the transcript
      overlay opening and closing).
- [ ] A command that prints the 16 ANSI slots draws in the host's palette and
      a plain prompt is the host's default foreground on the host's background;
      grepping the `ui` cartridge finds no post-compose pixel pass (the only
      paint path is `buffer.setCell` from the stored grid).
- [ ] `python3 -m unittest discover -s core/tests -p test_shell.py` — the
      `nvim` passage that draws a full screen and leaves the prompt intact
      passes against the painter (after the `needs` land).
- [ ] `bun --conditions=browser test tests/painter.test.tsx` prints the rows/s
      the painter sustains while a program redraws a full screen; that number
      and the after-transport number are recorded in this memo's `## Result`.
- [ ] Replacing the `ui` cartridge redraws the same shell screen from the grid
      (the replacement test in `core/tests/test_shell.py` passes).

```sh
cd /Users/feb/dev/sys/.claude/worktrees/the-ui-paints-the-grid
bun install --cwd builtin/ui --frozen-lockfile
bun run --cwd builtin/ui check
bun run --cwd builtin/ui test
bun --conditions=browser test builtin/ui/tests/painter.test.tsx
python3 -m unittest discover -s core/tests -p test_shell.py
cd /Users/feb/dev/sys                    # the lane is where the gate runs: see justfile
just check
just test
```

## Approach

Keep the `term` service shape so the swap is one commit: `read` stays for
replay, `frame`/`viewport`/`event` pass through, `resize` stays. Transport is
the existing JSON lines until the measurement says otherwise; rows as styled
runs, not cells, keeps it small. If it does not hold, the fallback is a compact
binary row encoding over the same op, not a new channel. The probe on
`work/the-ui-paints-the-grid` (committed in lane) builds the whole UI half and
is the baseline the implementer rebases the transport measurement onto.
