---
kind: work
description: "Command indicators coexist with the latest-tool footer and composer/transcript access"
status: open
subwork:
  - "[[the-gutter-shows-both-sides]]"
  - "[[the-sidebar-slides-over-the-shell]]"
---

# the-gutter-is-the-boundary

## Outcome

Command indicators can show agent and shell state beside the terminal while
preserving [[the-agent-surface-preserves-the-visible-shell]]: latest-tool footer,
no empty-tool placeholder, main/dynamic status and Ctrl+G/Ctrl+F access.
The gutter is an additional command boundary, not the whole application surface.

The grid belongs to [[the-terminal-is-drawn-from-pty]]. Agent text stays outside
its stream. Composer/transcript overlays preserve nvim's size and cursor.

## Check

- [ ] [[the-gutter-shows-both-sides]] is done.
- [ ] [[the-sidebar-slides-over-the-shell]] is done.

## Probe

Analyst probe, 2026-09-12, committed on lane
`work/the-gutter-is-the-boundary` (a1bb329): every parent Check box maps onto
a child Check box, and neither child consumes the other — `pty` already serves
per-command block data (`Command { id, command, exit, output, cwd }`,
see `builtin/pty/marks.rs`), and `builtin/ui/ui/terminal.tsx` already carries
the latest-tool footer, no-tool placeholder, main/dynamic status and
Ctrl+G/Ctrl+F/Escape surface both children build on. The runnable check is
`probe_gutter_split.py` in that lane.

## Approach

The gutter child owns command indicators; the historical sidebar child now owns
composer/transcript integration. The earlier requirement that no row appear
below the grid was superseded on 2026-09-12. Reserved footer height may reduce
the initial shell viewport, but opening a view must not resize the running PTY.
