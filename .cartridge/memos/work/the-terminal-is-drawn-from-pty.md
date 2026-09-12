---
kind: work
description: "The pty cartridge is the terminal emulator; the UI paints its grid and forwards input"
status: active
owner: "sys-work-2026-09-12/analyst-the-terminal-is-drawn-from-pty"
subwork:
  - "[[copy-mode-interacts-with-the-text]]"
  - "[[pty-owns-the-terminal-grid]]"
  - "[[pty-encodes-input]]"
  - "[[the-ui-paints-the-grid]]"
  - "[[the-embedded-terminal-is-gone]]"
---

# the-terminal-is-drawn-from-pty

## Outcome

Terminal emulation moves from OpenTUI's native embedded terminal into the `pty`
cartridge, under [[the-agent-surface-preserves-the-visible-shell]]. `pty` holds the grid, the
scrollback, the viewport and the row of every mark, and it turns key and mouse
events into shell bytes. The `ui` cartridge paints rows and forwards events. A
user notices nothing: every program that worked keeps working, the host theme
still shows through, and the shell still survives UI replacement, now with its
scrollback intact rather than replayed.

Scope is the emulator, its wire to the UI, the painter and the removal of the
old path. Indicators beside the grid belong to [[the-gutter-is-the-boundary]]; the latest-tool footer, main/dynamic status and composer/full transcript stay. No tool means no placeholder, and agent output never enters the shell stream.

## Check

- [ ] Every child work memo listed in `subwork:` is done.
- [ ] Every test in `core/tests/test_shell.py` passes against the painted grid,
      including the real `nvim` passage and the resize delta.
- [ ] Replacing the `ui` cartridge shows the same scrollback without `pty`
      replaying its output ring; the marker written before the swap is still on
      screen after it.
- [ ] `zirkle run pty '{"op":"screen"}'` returns the rows the user is looking at.
- [ ] No reference to `EmbeddedTerminalRenderable` remains under `builtin/ui`.

- [ ] Ctrl+G/Ctrl+F/Escape preserve the running editor's size and cursor, while the latest-tool footer and dynamic status remain usable.

## Approach

Children in order: the emulator and its ops, input encoding, the painter behind
the existing `term` service shape so the view swaps in one commit, then the
cleanup that deletes the old path and rewrites the documentation.
