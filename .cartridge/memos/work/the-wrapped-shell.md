---
kind: work
description: "The wrapped shell behaves as the user's own shell, for any program they run in it"
status: done
uses:
  - usage: "[[read-usage]]"
    when: ["Changing the pty cartridge, terminal emulation, resize, signals or reattach behaviour"]
---

## Current contract — 2026-09-12

The shell survives UI replacement and retains its own input, screen and cursor.
Agent output stays outside its byte stream under
[[the-agent-surface-preserves-the-visible-shell]]. Current UI reserves a fixed
footer, so compare resize deltas and editor size before/after view switches;
do not assume host height minus one status row. The grid migration is tracked
by [[the-terminal-is-drawn-from-pty]]. The checked delivery below is historical
and does not assert that migration is complete.

# the-wrapped-shell

## Outcome

A user cannot tell the wrapped shell from the one they would have started
themselves. Full-screen programs work: `nvim`, a pager, anything that takes the
alternate screen and raw input. The `pty` cartridge owns the PTY for its own
lifetime, so the shell and everything running in it survive UI replacement and
reload ([[shell-is-a-pty-cartridge]]); the UI draws the stream and nothing more.

Scope is the shell and its terminal: the PTY, its size, its signals, its output
ring and reattachment. What the agent prints into that stream belongs to
[[the-inline-agent]].

## Check

- [x] `test_a_full_screen_program_draws_and_leaves_the_prompt_intact` runs the
      real `nvim` where one is installed, types in insert mode, sees the text
      drawn, quits, and gets an answer from the shell afterwards. Where no
      editor exists it runs a self-contained program that takes the alternate
      screen and one raw key, proving the same passage.
- [x] `test_the_wrapped_shell_is_the_users_own_shell` reads `stty size` inside
      the shell, resizes the host by six rows and twenty columns, and reads it
      again: the shell moves by exactly that. Its width is the host's; its
      height is the host's less the status bar, which is why the test asserts
      the delta rather than a fixed size.
- [x] The same test traps `INT` in the shell, sends Ctrl+C, and sees
      `caughtsigint` — the signal reached the program, not the wrapper, which is
      still answering afterwards. Ctrl+D at the prompt then ends the shell and
      the host exits zero.
- [x] `test_replacing_the_ui_leaves_the_shell_and_its_scrollback` writes a marker
      to the shell, replaces the `ui` cartridge, waits for the surface to be
      redrawn, and finds the same host process, the marker still in the
      scrollback, and the shell still taking commands.

## Approach

`core/tests/test_shell.py`: three tests over one fixture, the same shape
`test_development.py` uses — a real PTY, the real `pty` and `ui` cartridges, no
model.

## Result

Everything here already worked; what did not exist was any test that would notice
if it stopped. The suite runs in about eleven seconds.

Two things the writing of it taught, worth knowing before changing this code.
The shell is not the size of the host terminal — it is the host less the status
bar, so an assertion on an absolute size is wrong and an assertion on the delta
is right. And every one of these paths is asynchronous: a resize travels
SIGWINCH, a relayout and an RPC before it reaches the shell, and a replaced
surface attaches its input handler on its own schedule. Asking once and failing
is a race; the tests ask again until the shell answers.
