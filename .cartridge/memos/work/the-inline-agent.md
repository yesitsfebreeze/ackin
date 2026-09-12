---
kind: work
description: "The agent is reached and answered without leaving the terminal"
status: open
subwork:
  - "[[the-palette-and-exit]]"
  - "[[the-context-inspector-opens-from-the-chat-editor]]"
uses:
  - usage: "[[read-usage]]"
    when: ["Changing the composer, the agent's output path into the shell, approvals or run control"]
---

## Current contract — 2026-09-12

Follow [[the-agent-surface-preserves-the-visible-shell]]. Replies and tool output
live in the full transcript, with only the latest tool in the footer. Ctrl+G
opens the composer, Ctrl+F opens the full transcript, and Escape from that view
returns to the editor without cancelling. The current test is
`test_an_approval_is_answered_and_the_reply_lands_in_the_transcript` in
`core/tests/test_shell.py`. The done checks below record the earlier delivery;
reply-into-scrollback and its old test name are superseded historical evidence.

# the-inline-agent

## Outcome

The agent is beside the terminal, not instead of it. The user reaches it without
leaving the shell, and its replies arrive in the same scrollback they were
reading. A run in flight never takes the shell away: the prompt keeps accepting
input while the agent works.

Scope is the composer, the agent's output path into the shell stream, approvals
and run control. The shell underneath belongs to [[the-wrapped-shell]]; what the
agent knows belongs to [[the-record-feeds-the-agent]]; where the tokens come
from belongs to [[models-reach-the-agent]].

## Check

- [x] `test_the_composer_opens_and_leaves_without_taking_the_shell`: the shell
      writes a marker, Ctrl+G switches the chip to CHAT with that marker still
      on screen, and Escape returns to SHELL with the shell answering again.
- [x] `test_an_approval_is_answered_and_the_reply_lands_in_the_scrollback`: the
      reply arrives in the shell's own scrollback, its inline bold markers
      rendered away and ANSI styling present. Heading hashes are kept on purpose
      and styled, so the test asserts the styling rather than their removal.
- [x] The same test sees the approval name its tool in the status bar, answers
      `y`, and reads back "the decision was allow" — the agent's own words for
      what it received.
- [x] `test_a_run_is_cancelled_and_the_shell_kept_taking_input_throughout`:
      Escape during a pending approval cancels the run and returns to the shell,
      which then answers a command.

## Approach

`core/tests/test_shell.py` gained a scripted agent: a Lua cartridge providing
`agent` and `router` that asks to use one tool on every run and, once answered,
emits the Markdown reply and a `done`. No model, no network.

## Result

Three tests over the same PTY fixture as [[the-wrapped-shell]], about fifteen
seconds for all six together.

One thing worth knowing before writing another Lua fixture in a Python test: the
fixture literal must be a raw string. Without that, Python eats the `\n` escapes
before Lua sees them and the cartridge fails to parse, which surfaces only as an
empty screen and `service 'ui' unavailable` in the log.
