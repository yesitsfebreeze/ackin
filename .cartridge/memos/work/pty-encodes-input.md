---
kind: work
description: "The UI sends key, mouse and paste events; pty encodes them by the terminal's current modes"
status: active
owner: "sys-work-2026-09-12/analyst-pty-encodes-input"
needs:
  - "[[pty-owns-the-terminal-grid]]"
---

# pty-encodes-input

## Outcome

The UI no longer turns a keystroke into bytes. It sends the event it received:
a key name with modifiers and any text, a mouse action with cell coordinates and
button, or a paste. `pty` encodes each one the way the running program asked
for, because the emulator knows the modes: application cursor keys, keypad mode,
bracketed paste, the active mouse protocol and encoding, focus reporting, and
the kitty keyboard flags when a program has enabled them. Raw `write` stays for
tools that already hold bytes.

## Check

- [ ] A Rust test sends the up arrow in normal mode and after the program
      enables application cursor keys, and reads the two different sequences.
- [ ] A test pastes text with bracketed paste off and on; the second is wrapped
      and the first is not.
- [ ] A test clicks a cell after a program enables SGR mouse reporting and reads
      the SGR sequence; before it, nothing is written.
- [ ] The `nvim` passage in `core/tests/test_shell.py` types through events,
      not bytes, and still sees its text drawn.

## Approach

The chosen emulator crate carries the mode state; the encoder is a table over
key name and modifiers, with the mode-dependent rows read from the emulator at
call time. `term` in the UI cartridge forwards OpenTUI's `KeyEvent`,
`MouseEvent` and paste bytes unchanged.
