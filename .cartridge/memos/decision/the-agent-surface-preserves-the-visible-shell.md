---
kind: decision
description: The visible shell retains its editor while a latest-tool footer and full transcript expose agent work
status: accepted
date: "2026-09-12"
decided_by: user
supersedes: "[[the-terminal-grid-lives-in-pty]]"
---

## Choice

The user's shell belongs to `pty` and survives UI replacement. Agent text never
enters its byte stream. The bottom area shows only the last tool call and its
readback; with no call it displays no placeholder. Preserve the main status bar
and dynamic status widgets. Ctrl+G opens the composer; Ctrl+F in agent mode opens
the full transcript. Escape from the transcript returns to the editor without
cancelling the run. Switching these views preserves the editor's size and cursor.
Clear covered glyphs before painting the transparent composer.

The agent expresses intent, consults environment-specific memos, acts through
one visible `shell` interface and verifies its readback. `memo` supplies knowledge
and recipes; imported JustDown material is ordinary memos, not another runtime
cartridge. Working context retains goals, evidence, decisions and effects on the
next action; detailed history stays retrievable. The agent's ability to inspect,
test and extend this program follows
[[the-agent-can-diagnose-and-extend-its-runtime]].

## Why

The user's requests in this session explicitly retain the latest-tool bottom
area, status widgets, Ctrl+G/Ctrl+F access and output visible outside nvim.
The sidebar-only, no-footer clauses of [[the-terminal-grid-lives-in-pty]] conflict
with those requirements. This decision supersedes that surface choice while
preserving the compatible engineering work: one terminal emulator owned by
`pty`, a UI grid painter, input encoding beside terminal modes, scrollback that
survives UI replacement, and sub-agent sessions without additional PTYs.

## Consequences

[[the-terminal-is-drawn-from-pty]] remains open. Grid migration must preserve
this footer and transcript contract. [[the-gutter-is-the-boundary]] may add
command indicators alongside it; a gutter does not replace status or tool output.
[[the-sidebar-slides-over-the-shell]] now owns composer/transcript integration;
its historical filename does not require a sliding sidebar.
[[sub-agents-share-the-terminal]] retains the collaboration goal and must expose
it through discoverable services without adding hidden execution tools.

Observed in `builtin/ui/ui/terminal.tsx` on 2026-09-12: a fixed footer, overlay
composer/transcript, separate agent output and `clearOverlay` already implement
this surface. The UI still embeds its own emulator; the grid migration is future
work, not a completed property. `core/tests/test_shell.py` names the reply test
`test_an_approval_is_answered_and_the_reply_lands_in_the_transcript`.
