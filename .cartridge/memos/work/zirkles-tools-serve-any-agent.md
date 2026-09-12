---
kind: work
description: "zirkle's tools are reachable from outside zirkle, by a command and by MCP"
status: open
level: 9
priority: P2
subwork:
  - "[[every-tool-is-one-command]]"
  - "[[an-mcp-server-carries-the-tools]]"
  - "[[headless-policy-approval-channel]]"
  - "[[a-tool-is-declared-by-its-memo]]"
---

# zirkles-tools-serve-any-agent

## Outcome

A coding agent that is not zirkle's own agent uses zirkle's tools — the same
`tool.*` keys, under the same policy, against the same record and the same
wrapped shell — by running a command and by speaking MCP. zirkle becomes usable
as tooling from inside another agent, not only as the agent it ships.

Two transports exist today and neither carries tools alone. `zirkle run <key>`
and `zirkle call <key>` reach any provided service, but the caller has to know
the key and hand-write the tool envelope
(`{"op":"call","context":{"session","run","call","cwd"},"input":{…}}`), and
nothing lists what a profile exposes. `zirkle launch` carries tools to an
external agent only by also proxying that agent's model traffic through
`builtin/proxy`; an agent that brings its own model cannot use it.

Scope is that gap: a tool-only surface, and an MCP server over it. It does not
change what any tool does, does not add tools, and does not touch the proxy's
model path. The enabled tool set stays the profile's
(`.zirkle/default/init.lua` lists `tool.shell` and `tool.memo` today); this
terminal exposes whatever the profile holds, including whatever
[[tool-dispatch-and-routines-are-graph-nodes]] adds later.

[[core-composes-and-the-cli-selects-services]] bounds the shape: the MCP
server is cartridge behavior beside `builtin/proxy`, not a widened
`core/main.rs` branch. `core/main.rs` keeps only the narrow launch policy it
already has.

## Check

- [ ] Every child work memo listed in `subwork:` is done.
- [ ] A fresh checkout reaches `tool.memo` and `tool.shell` from a plain
      command and from an MCP client, with `just check` and `just test` green.
