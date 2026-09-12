---
kind: work
description: "A run is a stream of declared event types that anything can post into, a priority event preempts the step and the run continues, and cartridges listen by type"
status: open
level: 10
subwork:
  - "[[an-event-declares-its-type]]"
  - "[[a-live-run-accepts-events-from-outside]]"
  - "[[a-priority-event-unblocks-the-step]]"
  - "[[a-listener-subscribes-to-event-types]]"
  - "[[a-stalled-step-raises-its-own-event]]"
uses:
  - usage: "[[read-usage]]"
    when: ["Changing what a run records, delivering something to an agent that is already working, or connecting a cartridge to what an agent is doing"]
---

# the-run-is-a-stream-of-typed-events

## Outcome

A run is not a conversation with extra bookkeeping around it; it is one ordered
stream of typed events, and a conversational message is one type among several.
Three things follow, and they are why this is worth doing at all:

Anything can post an event into a run that is already working. A run today is
closed while it runs: it is started, answered and cancelled, and nothing else
reaches it. A post is the third door, and it is durable whether or not a run is
live.

A posted event may be more important than the step the run is in. A priority
event cuts the model stream or the cancellable tool the run is waiting on,
keeps what that step produced, and **continues the same run** with the new
information. That is the unblock: an agent that has been grinding on something
too small for the time it is taking gets told so by another agent and carries on,
rather than being killed and restarted from nothing.

A cartridge can listen to a type. Tools connect to each other through the stream
instead of through a transcript nobody else can parse, which is what makes the
stall signal in [[a-stalled-step-raises-its-own-event]] possible without teaching
every tool about every other.

This is the delivery half of coordination. [[the-swarm-talks-on-a-board]] owns
addressing, the channels and the protocol agents talk in; this memo owns what
happens when a line has to reach an agent that is busy, and what a run's record
is made of. The visible shell stays the user's, and an event is never an
execution path: a listener that wants something done posts, it does not act.

| Work | Estimate |
| --- | --- |
| [[an-event-declares-its-type]] | 1d |
| [[a-live-run-accepts-events-from-outside]] | 2d |
| [[a-priority-event-unblocks-the-step]] | 2d |
| [[a-listener-subscribes-to-event-types]] | 1d |
| [[a-stalled-step-raises-its-own-event]] | 1d |

## Check

- [ ] Every child is done: a `tool.memo` list of kind `work` shows each memo
      named in this memo's `subwork` with `status: done`.
- [ ] `just test` carries an unblock probe: one run is held in a slow tool, a
      second session posts a priority event, and the first run cuts that step,
      carries the event into its next model round and reaches `completed` — one
      run from start to finish, with no second run started and no `cancelled`
      phase anywhere in its journal.
- [ ] The same probe replays that run's stream by type and shows the cut step,
      the posted event and its sender in order.
- [ ] For a run that nobody posts to and a profile with no listener, the journal
      and the projected request are byte-identical to today's for the same
      inputs.

## Approach

Observed 2026-09-12, reading `builtin/agent/lib.rs`, `builtin/harness/working.rs`
and `builtin/sessions/main.rs`. Most of this shape is already in the tree and
unnamed.

The journal is already a typed stream: `record(kind, run)` mints `{v,kind,run}`
(`builtin/agent/lib.rs:191`) and the run writes `message`, `run_started`,
`tool_started`, `tool_finished`, `model_turn_started` and `run_finished` into it
through one checkpoint. The projection already keeps exactly one of those:
`working.rs:26` skips every record whose `kind` is not `message`. So "an event
stream of types, of which `message` is one" is a description of what is stored
today, not a rewrite of it.

Cancellation is already surgical and already keeps what it cut: the model stream
races a cancel watch and returns its visible partial (`builtin/agent/stream.rs:116`),
and the active tool call is cancelled by its exact identity (`lib.rs:454`). What
is missing is that both signals mean only one thing — end the run.

Listeners already exist at the host: a cartridge registers them over the wire
(`core/cartridge.rs:407`, `core/sdk.rs:279`) and the run already publishes each
stage through `emit` (`lib.rs:561`), which names the kind but is a notification
nobody subscribes to by type.

And there is already a durable inbox with no reader: `send{id,from,text}` appends
to a session's `mailbox` buffer and `mailbox{id}` reads it back
(`builtin/sessions/main.rs:661`), used by nothing in the tree.

What is missing is small and specific: a declared vocabulary with a projection
rule per type, a way in for a writer that is not the run task, a signal that means
continue rather than end, and a subscription.

## Spec

The run task stays the only writer of its own stream. Everything from outside is
queued on the run's control and drained by the run at a point it chooses. This is
not ceremony: the checkpoint is guarded by `expected_revision` (`lib.rs:540`), and
a second writer would either lose events to a conflict or land one between an
assistant tool call and its result, which makes the run unprojectable.

Preemption is a separate signal from cancellation, never a flag on it.
`cancel` ends a run and must keep ending it; an unblock keeps the run. Sharing one
channel for both would make the difference a matter of ordering, and the failure —
a run that dies when it should have carried on — is exactly the one this memo is
here to remove.

A type declares whether it reaches the model and in what frame. The projector
asks the type; it does not grow a branch per kind. That is what lets an
explanation, a memory or a notice ride the same stream as a message without the
harness learning about each one, and it is the seam a later type is added at.

Order of work is the vocabulary, then the way in, then preemption, then
subscription, then the stall signal. The first two are independently useful — a
durable post that a run picks up at its next boundary is already the board's busy
case answered.
