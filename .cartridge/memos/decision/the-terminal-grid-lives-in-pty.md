---
kind: decision
description: The terminal emulator is Rust code in the pty cartridge, the UI paints its grid, and a two-column gutter is the only boundary between agent and shell
status: superseded
date: "2026-09-12"
supersedes: "[[shell-is-a-pty-cartridge]]"
decided_by: user
---

## Superseded on 2026-09-12

Follow [[the-agent-surface-preserves-the-visible-shell]] and its successor for the current contract. The original
choice and rationale below are historical; they are not instructions to remove
the latest-tool footer, status widgets or full transcript. PTY ownership and
UI-independent shell lifetime remain required.

## Choice

The `pty` cartridge owns the terminal emulator: the screen grid, the scrollback,
the viewport, the absolute row of every shell-integration mark, and the encoding
of key, mouse and paste events into the bytes the shell receives. The `ui`
cartridge paints that grid and forwards events; it embeds no emulator of its own.
OpenTUI stays as the layout, text, markdown and input toolkit. This is the only
terminal integration: no tmux, no multiplexer, no second emulator anywhere.

The surface is the shell with a two-column gutter on its left. The left column
is the agent's, the right column the shell's; each shows one icon per command
block, pinned at the top row while the block is scrolled past. Nothing is drawn
below the shell. Ctrl+G slides an agent sidebar in from the left over the grid,
which is drawn shifted right and clipped, never resized; Escape returns to the
shell. The sidebar holds the full transcript, a slot column for panels, the
composer and one status row. Sub-agents are sessions in the same agent process
that talk through a mailbox; they never get a shell of their own.

## Why

Brainstormed on 2026-09-12. OpenTUI's `EmbeddedTerminalRenderable` is a prebuilt
native library exposing only the visible screen and a relative scroll; without a
viewport offset, gutter icons cannot be aligned to rows through scrollback, and
adding that binding means patching upstream with a toolchain this workspace does
not have. An emulator in `pty` gives exact rows for marks, a screen the agent can
read with the same grid the user sees, key encoding beside the terminal modes it
depends on, and scrollback that outlives UI replacement, which is the rule of
[[shell-is-a-pty-cartridge]] carried one step further.

tmux would have supplied screen and keys, but adds a hard dependency, a nested
server when the user already runs tmux, and swallows OSC 133 marks. A side
split or a bottom drawer costs the shell width or height on every toggle; the
gutter costs two columns once, and the shifted sidebar costs nothing the shell
can notice.

## Consequences

Work: [[the-terminal-is-drawn-from-pty]], [[the-gutter-is-the-boundary]],
[[sub-agents-share-the-terminal]].

`pty.print` and the reply-into-scrollback path described in
[[shell-is-a-pty-cartridge]] are retired: agent text lives in the sidebar and
never enters the shell stream. The status bar, the mode chips and the latest-tool
preview go with it. The sentence in [[the-vision]] about replies landing in the
scrollback is superseded by the sidebar.
