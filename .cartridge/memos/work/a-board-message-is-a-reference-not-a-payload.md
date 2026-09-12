---
kind: work
description: "Agents exchange pointers into the shared record, so a swarm's messages stay small while its context stays complete"
status: open
level: 11
estimate: 1d
needs:
  - "[[the-agents-chat-through-one-tool]]"
---

# a-board-message-is-a-reference-not-a-payload

## Outcome

A message names where something is, not what it says. A board line carries
references — a memo path, a session id, a channel and sequence, a file path and
line, a run id — and the receiver resolves what it needs through the tools it
already has. Pasted file content, restated plans and quoted transcripts are
refused by the line cap rather than tolerated, so the swarm's token cost grows
with the number of things said and not with the size of what they are about.

The shared context is the record and the store, both already readable by every
agent. The board carries the pointers into them.

## Check

- [ ] A reference in a line resolves for another session: given a posted memo
      path and a channel sequence, a second agent reads both through existing
      tools without further messages.
- [ ] A line that inlines a file body exceeds the cap and is refused; the same
      information posted as a path and a line range is accepted.
- [ ] The protocol memo states the reference forms, and a probe shows an agent
      following one to the thing it names.
- [ ] Measured on one recorded multi-agent task, the board's total posted bytes
      are a small fraction of the bytes the referenced material would have cost;
      the result records both numbers.
- [ ] `just check` and `just test` pass.

## Approach

Observed 2026-09-12. Everything a message would want to quote is already
addressable: memos are `<kind>/<name>.md` in a record every enabled cartridge
shares, `tool.memo` resolves and reads them, sessions and runs have ids, and
channel lines will have sequence numbers from
[[the-board-is-channels-of-lines]]. What is missing is the discipline and its
enforcement point — without a cap, a model will paste, and a swarm's cost becomes
quadratic in what it has read.

## Spec

Define the reference forms once, in the protocol memo, and keep them the ones the
system already uses: `[[memo-name]]` for the record, `session:<id>`,
`run:<id>`, `<channel>#<seq>`, `path:line`. No new resolver: each form is
already resolvable by a tool the agent has.

The cap is the enforcement. A line that does not fit is refused with the cap in
the message, which teaches by failing at the moment of the mistake rather than by
a prompt instruction the model may skip. Where a long payload is genuinely
needed, the answer is to write it as a memo and post its name — which also makes
it durable and reviewable instead of stranded in a channel.

This is the same economy the harness already applies to context: bounded rendered
blocks plus retrievable detail. The board inherits it rather than inventing a
second policy.
