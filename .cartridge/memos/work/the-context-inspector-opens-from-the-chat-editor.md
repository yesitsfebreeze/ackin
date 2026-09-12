---
kind: work
description: "The /context inspector and the transcript have an entry point in the shipped terminal feature"
status: open
level: 11
priority: P2
estimate: 1d
---

# The context inspector opens from the chat editor

## Outcome

The user can open the harness context inspector from the running terminal, and
can still reach it and the transcript while a full-screen editor owns the shell.

`builtin/harness/README.md` documents this as shipped: "The chat editor's
`/context` command (or `Ctrl+O`, including during a run) opens a full-screen
inspector", with Up/Down selection, PgUp/PgDn scrolling, `r` to refresh and
Escape to close. It is not shipped. Verified on branch
`terminal-integration-and-native-memos`, 2026-09-12, while probing
[[a-turn-carries-one-id-through-host-lua-and-bun]]:

- The backend exists. `agent {op:"context"}` is served at
  `builtin/agent/lib.rs:264` and calls `harness {op:"inspect"}`.
- The frontend does not. `builtin/ui/ui/terminal.tsx` binds Ctrl+G (composer),
  Ctrl+F (transcript) and Ctrl+Y (yank) and nothing else — there is no Ctrl+O.
- No slash-command parsing exists anywhere under `builtin/ui/`, so `/context`
  typed into the composer is sent to the model as prompt text.

So the inspector has no entry point at all, not merely a missing shortcut. The
transcript half of the claim does hold: agent output is drawn separately from
the shell, so Ctrl+G and Ctrl+F work while an editor owns the terminal.

Either ship the entry point or correct `builtin/harness/README.md`; the record
and the binary must not disagree.

## Check

- [ ] Ctrl+O from the running terminal opens the context inspector, including
      while a run is in flight.
- [ ] `/context` typed in the chat composer opens the same inspector and is not
      sent to the model as a prompt.
- [ ] Both work while a full-screen editor holds the shell, and Escape returns
      to the editor with its screen and cursor intact.
- [ ] `builtin/harness/README.md` describes exactly what the binary does.
