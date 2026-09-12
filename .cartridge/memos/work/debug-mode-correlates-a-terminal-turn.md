---
kind: work
description: "The agent enters debug mode to test and diagnose its capabilities"
status: open
level: 10
priority: P1
estimate: 2d
needs:
  - "[[the-agent-can-discover-its-own-program]]"
subwork:
  - "[[debug-mode-opens-and-closes-from-the-shell]]"
  - "[[every-enabled-tool-ships-a-contract-probe]]"
  - "[[tool-probes-run-and-locate-a-failing-provider]]"
  - "[[a-turn-carries-one-id-through-host-lua-and-bun]]"
---

# The agent tests and diagnoses its own runtime

## Outcome

The agent can enter and leave debug mode itself through the visible shell,
inspect the running program, form a hypothesis, exercise capabilities and
compare observed behavior with their contracts. It can validate the complete
enabled tool set and locate a failure in its cartridge. Correlated diagnostics
provide evidence for this loop. The user can continue to see the agent's intent,
commands and results outside a full-screen editor.

## Check

- [ ] Every child is done: [[debug-mode-opens-and-closes-from-the-shell]],
      [[every-enabled-tool-ships-a-contract-probe]],
      [[tool-probes-run-and-locate-a-failing-provider]] and
      [[a-turn-carries-one-id-through-host-lua-and-bun]].

## Approach

Clarified by the user on 2026-09-12: the debugger is an agent-operated testing
and diagnosis workflow. Existing status/tail/list, shell readback, cartridge
metadata, /context and Ctrl+O are ingredients. The earlier proposal covered
mostly logging; retain that evidence channel within the executable workflow.
Use the program map and cartridge-provided probe memos, keeping shell and memo
as the model-facing interface. Test the loop with a deterministic local provider.

Choice: [[the-agent-can-diagnose-and-extend-its-runtime]]. Extension: [[the-agent-can-extend-and-verify-a-cartridge]].
Audit: [[runtime-audit-2026-09-12]].

### Split, 2026-09-12

An analyst pass probed this on lane `work/debug-mode-correlates-a-terminal-turn`
and split it. What the five original Check boxes wanted is four independent
contracts, not one: the lifecycle, the discovery of what to probe, running the
probes and diagnosing from them, and the correlated evidence channel. The last
is independent of the first three. `estimate: 2d` covered at most one.

The probe is committed on that lane as `165cd6d`, and the first child continues
it: `zirkle run` now serves the profile socket for as long as its foreground call
lives, so `zirkle status` and `zirkle call` from the wrapped shell reach the
**running** host instead of loading a second copy of the profile. Before it,
only `zirkle daemon` bound the socket — `core/main.rs`'s `Command::Run` never
called `socket::serve` — so "through the visible shell" had nothing to talk to.
`core/tests/test_run_socket.py` covers it and was seen failing without the
change.

What the probe found, and which child carries it forward:

- `zirkle list` already resolves every tool key to its provider (`tool.shell <-
  pty`, `tool.memo <- memo`) and `zirkle status` already reports every fiber with
  state, injections, provides and error — most of "enumerate every enabled tool
  and its backing services" is rendering, not new bookkeeping.
- Calling a tool over the socket is refused: `tool.shell` answers `invocation
  context required`, `memo` answers `trusted memo cwd required`. Any probe
  runner has to supply what the agent's own dispatch supplies.
- No cartridge ships a probe memo, and no kind declares one.
- No correlation identifier exists anywhere in `core/` or `builtin/` outside
  test names and doc comments; the whole evidence channel is `just run`
  truncating UI stderr into one `.zirkle/logs/ui.log` (`justfile:81-83`), with
  cartridge stderr inherited onto the same stream (`core/cartridge.rs:242,283`).
