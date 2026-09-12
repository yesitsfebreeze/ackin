---
kind: work
description: "Debug discovery lists every enabled tool with its backing service and a runnable contract probe its cartridge ships"
status: open
level: 11
priority: P1
estimate: 2d
needs:
  - "[[debug-mode-opens-and-closes-from-the-shell]]"
---

# Every enabled tool ships a contract probe

## Outcome

Debug discovery answers one question completely: what can this agent do, who
provides it, and how is that claim checked. It enumerates every enabled agent
tool with the service backing it, and for each one a runnable contract probe
shipped by the providing cartridge. A tool with no probe, and a capability that
is skipped or unavailable in this environment, is reported as **unverified** —
never as passed, never silently omitted.

Half the enumeration already exists and should be reused, not rebuilt: `zirkle
list` resolves every injected key to its provider, printing `tool.shell <- pty`
and `tool.memo <- memo` for the default profile, and the effective tool set is
one explicit table in `.zirkle/default/init.lua` that feeds both the agent's
injections and its dispatch list. `zirkle status` adds the live per-fiber state.

The missing half is the probe itself. No cartridge ships one today: of
`builtin/{agent,fs,harness,memo,policy,sessions,shell,ui}` with a `.zirkle/memos/`
record, none holds a probe memo, and there is no declared kind for one. A probe
travels with the cartridge that makes the claim, so adding a tool adds its probe
in the same folder — discovery must not carry a central list of what to check.
The program map from [[the-agent-can-discover-its-own-program]]
(`builtin/memo/.zirkle/memos/note/program-map.md`,
`program-cartridge-contracts.md`, `program-checks.md`) is the description to
attach probes to.

Scope: declaring and discovering probes, and reporting what is unverified.
Executing them against fixtures is
[[tool-probes-run-and-locate-a-failing-provider]].
