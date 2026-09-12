---
kind: work
description: "One board tool serves post, read, watch and roster to every agent, and the protocol it teaches is terse by contract"
status: open
level: 11
estimate: 2d
needs:
  - "[[the-board-is-channels-of-lines]]"
  - "[[an-agent-is-one-lookup-from-the-roster]]"
---

# the-agents-chat-through-one-tool

## Outcome

Agents talk through one tool. A board cartridge provides `tool.board` with
`post`, `read`, `watch`, `unwatch`, `channels` and `roster`, so every profile
that injects the tool surface gives it to its agent and serves it to outside
clients over MCP without a second registration. Posting to a channel an idle
session watches wakes that session with the unread lines in its prompt; posting
to a busy one leaves the lines for its next read, except for a priority line,
which [[the-run-is-a-stream-of-typed-events]] carries into the run it is
already in.

The protocol is part of the tool, not of a system prompt: the cartridge ships the
routine memo that teaches it, and `describe` states the rules the schema can
enforce — one claim per line, a length cap, an explicit addressee, no restated
context. Terse is a contract the tool holds, so it survives a prompt rewrite.

## Check

- [ ] With the board cartridge enabled and nothing else changed, the agent's tool
      list contains `board` and `zirkle mcp` serves it; with the cartridge
      disabled, neither does and no board memo appears in the record.
- [ ] A post to a channel watched by an idle session starts one run for that
      session whose prompt carries the unread lines, and a second post during
      that run does not start a second run.
- [ ] `watch`/`unwatch` change what a session is woken by, and survive a restart.
- [ ] A post over the length cap is refused with a message naming the cap, and an
      over-long post never reaches a channel.
- [ ] The board's routine memo is discoverable through `tool.memo` resolve for
      the situation of messaging another agent, and it is absent from the record
      when the cartridge is disabled.
- [ ] Two sessions posting to one channel in the same second both appear, in
      sequence order, with distinct `from` values.
- [ ] `just check` and `just test` pass.

## Approach

Observed 2026-09-12. The tool surface is already a glob: `.zirkle/default/init.lua`
gives `agent` and `memo` `inject = {"tool.*"}`, and `.zirkle/mcp/init.lua` gives
`mcp` the same, so a cartridge that provides `tool.board` is in the agent's
dispatch list and in the MCP server with no other edit. Every tool cartridge
shares the `describe`/`call`/`cancel` envelope with a trusted `context` beside
the model's `input` (`builtin/memo/src/service.rs:69`). A cartridge can ship its
own `.zirkle/memos/`, merged read-only into the workspace record, which is how a
routine memo arrives and leaves with the cartridge. Waking a session is the agent
cartridge's contract, already claimed by
[[the-agent-spawns-wakes-and-owns-sub-agents]]; this memo only gives it a channel
trigger beside the existing mailbox one.

## Spec

The board cartridge is thin: it holds the protocol memo, the `describe` text and
the tool envelope, and it reaches the channels through the sessions service. State
stays in one place — the store — so `tool.board` is a surface, not a second
database.

Wake is a subscription, not a poll: a post emits its channel as the event target
(the shape `send` already uses), the agent cartridge watches the channels its
sessions watch, and an idle session's next run is started with its unread lines
rendered in the prompt. One wake per session per idle period, coalescing the lines
that arrived, so a burst of chatter is one run and not ten.

The protocol memo is what the user asked for as caveman language, written as a
routine: one claim per post, address by session id or channel, state result or
blocker, never restate what the roster or the record already says, link instead
of quoting ([[a-board-message-is-a-reference-not-a-payload]]). What the schema can
check, the schema checks — cap, required addressee, one line. What it cannot, the
memo teaches and the log shows.
