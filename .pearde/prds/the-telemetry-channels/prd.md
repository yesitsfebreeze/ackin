---
state: open
origin: requested
priority: 72
complexity: 0
blast-radius:
needs:
  - the-wire
---

# The telemetry channels

Every cartridge publishes what it is doing, and any other cartridge can listen.

The requirement as it arrived, verbatim:

> "the base cartridge should have a telemetry system where it can publish and
> broadcast events in channels essentially. That way we can get a good idea from
> other plugins of what is going on, what is not working, errors, all of that. So
> it really lends itself to agentic swarm engineering. The harness that we are
> building here is basically comprised of many cartridges that do a very specific
> job very well. Therefore the communication layer must be replayable,
> deterministic, accurate, fast, and small. Thus I thought of a JSON ledger of
> events that we can just stream into other agents. So one agent can subscribe to
> a channel and then he can listen to all the events in that channel. He can
> unsubscribe from it so everybody knows, in essence, what is going on with the
> other cartridge. And this is the end goal but the base system must make this as
> easy as possible."

Restated as a contract:

- A cartridge publishes an event to a named channel. It does not know who is
  listening, and it costs it nothing to find out that nobody is.
- Another cartridge subscribes to a channel by name and receives every event
  published to it, in order, from the point it subscribed. It unsubscribes, and
  both the subscription and its end are themselves events on the channel — so
  every participant knows who is watching.
- The stream is a JSON event log: one object per line, append-only, ordered.
- **Replayable and deterministic.** The same log replayed twice produces the
  same state twice. A subscriber that crashed and came back is handed the same
  events in the same order and ends up where it was. This is the requirement,
  not a property that would be nice to have.
- **Accurate.** An event that was published appears exactly once. An event that
  was not published does not appear.
- **Fast and small.** Publishing sits on the hot path of every cartridge, so
  the cost with no subscriber must be near zero, and the envelope must not
  dwarf the payload it carries.

Errors are events. A cartridge that fails publishes the failure on its channel
rather than only returning it to whoever called — the caller is not the only
party that needed to know.

This is the base layer, not the consumer. The end use is agentic: an agent
subscribes to a channel and reads another cartridge's work live — what it did,
what broke, what it is waiting on — so a swarm of cartridges is legible from
outside. This PRD builds what makes that trivial: the publish call in the SDK,
the channel registry, the subscribe and unsubscribe protocol on the wire, and
the event envelope. It does not build the agent.

In scope, and to be settled here: whether events travel only along birth pipes
— a node knows its children, so a subscription routes up the tree and back down
— or through one host-resident bus every node reaches. The first keeps the
vision's "no discovery, no broker". The second is simpler and is a broker.
Decide it, and put the reasoning in a memo.

Not in scope: retention policy, on-disk persistence beyond what replay needs,
and any query language over the log. A consumer that wants those reads the
stream and builds them itself.

At the end, a cartridge publishes an event in one line with no setup, a second
cartridge subscribes to that channel by name and receives everything on it in
order, a replay of that log reaches the same state, and unsubscribing is
visible to everyone on the channel.

A word on names: on this board **ledger** already means the registry of every
cartridge installed on this machine — see [[the-ledger]]. The JSON ledger of
events asked for above is called the **stream** here, so the two never collide.
[[the-wire]] is what carries it.
