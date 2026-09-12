---
kind: work
description: "Anything can post an event into a run that is already working; the run task stays the only writer and drains it at a boundary"
status: open
level: 11
estimate: 2d
needs:
  - "[[an-event-declares-its-type]]"
---

# a-live-run-accepts-events-from-outside

## Outcome

A run that is already working can be told something. `post{session, type, text,
from}` appends one event to that session's stream from outside the run: from
another agent, from a listening cartridge, from the user, from the shell. It is
the third door beside starting a run and answering an approval, and unlike both it
does not have to wait for the agent to be idle.

The run task remains the only writer of its own journal. A post is queued on the
run's control and drained by the run at a point where the stream is whole —
after a reply, or after a complete tool group — written as one journal record
carrying its sender and its arrival, and carried into the next model round marked
as having arrived mid-run rather than as something the user said at the start.

A post to a session with no live run is not an error and is not lost: it is
durable, and the session's next run begins with it. Whether that post also *wakes*
an idle session is not this memo's contract; it belongs to the spawn and board
work.

## Check

- [ ] A post during a run appears exactly once in that run's journal, with its
      sender, and the next assembled request carries it.
- [ ] A post never lands between an assistant message with tool calls and its
      results: a probe posting continuously through a tool loop leaves every
      projection of that run valid at every boundary.
- [ ] Two posts arriving concurrently both land, in arrival order, and no
      checkpoint conflict drops either.
- [ ] A post to a session whose run has finished is durable across a restart, and
      the next run's first request carries it.
- [ ] A post of a non-projecting type is stored on the stream and never reaches the
      model.
- [ ] A post naming a session that does not exist fails explicitly and writes
      nothing.
- [ ] `just check` and `just test` pass.

## Approach

Observed 2026-09-12. The agent's op surface is `start`, `status`, `context`,
`cancel`, `answer` (`builtin/agent/lib.rs:261`) — there is no way in for a third
party. The machinery for one is already there: `Control`/`Live` carries
out-of-band state to a running task (the pending approval and the active call,
`lib.rs:113`), and the checkpoint is guarded by `expected_revision` with the run
task as sole writer (`lib.rs:540`), which is the reason the queue exists rather
than a second writer.

Durability for the idle case is already implemented and unused: `send{id,from,text}`
appends a JSON line to the target session's `mailbox` buffer and `mailbox{id}`
lists it back (`builtin/sessions/main.rs:661`), with no reader anywhere in the
tree. A post to an idle session is that path with a type on it; this memo gives it
its first consumer.

Boundaries are already computed for exactly this shape: `working.rs:15` finds the
ends of replies and of complete tool groups when deciding where a summary may cut.
The drain point and the compaction point are the same question asked twice.
