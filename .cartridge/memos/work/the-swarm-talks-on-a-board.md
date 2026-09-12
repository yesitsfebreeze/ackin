---
kind: work
description: "A message board with channels, a roster, a terse referencing protocol and a followable log, so many agents share one context cheaply"
status: open
level: 10
subwork:
  - "[[the-board-is-channels-of-lines]]"
  - "[[an-agent-is-one-lookup-from-the-roster]]"
  - "[[the-agents-chat-through-one-tool]]"
  - "[[a-board-message-is-a-reference-not-a-payload]]"
  - "[[the-board-log-replays-who-did-what]]"
uses:
  - usage: "[[read-usage]]"
    when: ["Making agents talk to each other, adding a channel or a board op, or deciding what a message may carry"]
---

# the-swarm-talks-on-a-board

## Outcome

Agents and sub-agents coordinate on a message board instead of in each other's
prompts. Channels carry short lines; a roster answers who is doing what in one
lookup; messages carry references into the shared record rather than copies of
it; and one ordered log replays who did what. The result is a planning force with
many tools and a small context: an agent's per-turn cost is a bounded roster
block plus the lines it has not read, not the history of everyone else.

This is the coordination half of [[sub-agents-share-the-terminal]]: that memo
owns spawning, ownership of the visible shell and the panel that shows children;
this one owns how they talk. The visible shell stays the owner's, and a board
post is never an execution path.

| Work | Estimate |
| --- | --- |
| [[the-board-is-channels-of-lines]] | 1d |
| [[an-agent-is-one-lookup-from-the-roster]] | 1d |
| [[the-agents-chat-through-one-tool]] | 2d |
| [[a-board-message-is-a-reference-not-a-payload]] | 1d |
| [[the-board-log-replays-who-did-what]] | 2d |

## Check

- [ ] Every child is done: a `tool.memo` list of kind `work` shows each memo
      named in this memo's `subwork` with `status: done`.
- [ ] `just test` carries a swarm probe in which one planner and two workers
      complete a task through the board alone: the probe asserts the planner
      read no worker transcript and that every handoff line carries a reference.
- [ ] The same probe replays its log through `follow` as an ordered story naming
      each actor, and records the planner's rendered prompt size with two and
      with eight workers, both within the configured roster and unread caps.
- [ ] With the board cartridge removed from the profile, that probe's tool-list
      assertion shows no `board`, a record index shows no board memos, and a
      composed prompt shows no roster block.

## Approach

Observed 2026-09-12, reading `builtin/sessions/main.rs`. More exists than the
board needs to invent. A session already records a `parent` at `create` and
carries agent state including `phase`, which the store guards during a run
(`:505`, `:541`); `list` returns every session (`:503`); and a durable mailbox is
already implemented — `send{id,from,text}` appends a JSON line to the target
session's `mailbox` buffer and notifies with that session as the event target,
`mailbox{id}` reads it back (`:666`, `:682`). The tool surface is a glob, so a
cartridge providing `tool.board` reaches the agent and MCP with no second
registration (`.zirkle/default/init.lua`, `.zirkle/mcp/init.lua`). The harness
already renders bounded live blocks from a service named by config
(`builtin/harness/README.md:136`).

What is missing: a channel name that is not a session id, a sequence number, a
read cursor, one roster projection, a protocol the tool enforces, and one
ordering across sessions.

## Spec

The board is a cartridge over the sessions store, not a new store and not an
external service. Two decisions follow from that, stated here so the children can
assume them.

The board is local. An existing chat product would give a human UI for free, but
it would put auth, network latency, rate limits and a third party between agents
that share a machine, and it cannot hold the ordering that
[[the-board-log-replays-who-did-what]] needs. So the substrate is `sessions`, and
a bridge that mirrors a channel into an external chat for human reading is a
separate, optional cartridge — worth building when watching matters more than
coordinating, and not before.

Terseness is enforced, not requested. The line cap in
[[the-board-is-channels-of-lines]] and the reference forms in
[[a-board-message-is-a-reference-not-a-payload]] are what make the protocol hold;
the routine memo the board cartridge ships teaches the rest. A rule that lives
only in a prompt is a rule the next prompt rewrite loses.

Order of work is channels, then roster, then the tool, then the protocol, then the
log. The first two are cheap and independently useful — a roster over today's
mailbox already answers who is doing what.
