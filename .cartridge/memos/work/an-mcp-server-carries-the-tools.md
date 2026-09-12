---
kind: work
description: "An MCP server exposes zirkle's tools to any MCP client, over the tool command"
status: open
level: 10
priority: P2
estimate: 2d
needs: ["[[every-tool-is-one-command]]"]
---

# an-mcp-server-carries-the-tools

## Outcome

Any MCP client — Claude Code, Codex, an editor — reaches zirkle's tools by
configuring one command. The server lists the profile's `tool.*` keys as MCP
tools, named and schema'd from each tool's own `describe`, and forwards a call
to the tool surface [[every-tool-is-one-command]] delivers, so there is one
path to a tool and not two. A tool's `error:true` arrives as an MCP tool
error, not a transport failure; a cancelled MCP call reaches the tool's
`cancel` op with the same invocation identity, as `builtin/proxy` already does
on drop.

This is the reason for the terminal: another coding agent keeps its own model
and its own loop, and still gets zirkle's shell, record and policy. It is the
complement of `zirkle launch`, which gives an external agent zirkle's models.

One fork is open and belongs to the probe, not to the plan: a server that
shells out per call loads the whole profile each time — static and isolated,
which is the point, but slow — while one that talks to a running daemon over
`zirkle call` is fast and needs a daemon. Measure both before choosing; a cold
`zirkle run` on the default profile spawns every cartridge in it.

Per [[core-composes-and-the-cli-selects-services]] this lands as a cartridge
beside `builtin/proxy`, not as a widened `core/main.rs`.

## Check

- [ ] An MCP client configured with the documented command lists zirkle's tools
      and a call returns the tool's content; verified with
      `claude mcp add zirkle -- <command>` and a `memo` list call in a session.
- [ ] The tool list and its schemas come from `describe`: removing a key from
      `.zirkle/default/init.lua` removes that MCP tool with no Rust change.
- [ ] A tool returning `error:true` arrives as an MCP tool error carrying the
      tool's content, and the client can continue.
- [ ] Cancelling an in-flight MCP call invokes the tool's `cancel` op with the
      same context, proven by a test in the cartridge's own tests.
- [ ] A README in the cartridge carries a config snippet a fresh checkout can
      paste, and the per-call cost of the chosen transport is recorded there.
- [ ] `just check` and `just test` pass.
