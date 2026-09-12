---
kind: work
description: "A sub-agent session records its parent, and messages travel through a durable mailbox on the sessions cartridge"
status: open
---

# sub-agent-sessions-record-parent-and-mailbox

## Outcome

A sub-agent is a session with `parent` recorded on it. `sessions create{parent}`
records the parent on the created session; `sessions get`/`list` expose it and
it survives restart. Messages to a session travel through a mailbox on the
sessions cartridge: `sessions send{id,from,text}` appends one message line to
the target session's mailbox buffer, notifies, and persists; `sessions
mailbox{id}` lists the messages back in order. The composer and the agent call
the same ops, so a message to an idle agent parks in the mailbox for the next
run; waking that agent is the agent cartridge's contract, not this one.

## Check

- [ ] `sessions create{parent}` records the parent on the session; `get`/`list`
      return it and the session's snapshot file contains it.
- [ ] `sessions send{id,from,text}` appends one message and `sessions
      mailbox{id}` lists messages back in order with `from`, `to` and `text`.
- [ ] Reloading the store returns the same parent and mailbox messages.
- [ ] `sessions send` to a missing session is refused without changing the store.
