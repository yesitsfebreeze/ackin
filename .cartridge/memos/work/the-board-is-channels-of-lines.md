---
kind: work
description: "The sessions store serves named channels of append-only message lines, and the per-session mailbox becomes one of them"
status: active
owner: "sys-38/implementer-the-board-is-channels-of-lines"
level: 11
estimate: 1d
---

# the-board-is-channels-of-lines

## Outcome

A message board lives in the sessions cartridge: named channels, each an ordered
append-only sequence of short message lines that survives restart. `post` appends
one line to a channel and returns its sequence number; `read` returns a channel's
lines from a sequence onward, newest-bounded by a limit; `channels` lists every
channel with its last sequence and last activity. Every line carries `from`, `ts`,
`seq` and the run it came from, so a reader can always ask who said it and when.

The existing per-session mailbox is the direct-message case of this store, not a
second implementation: a session's mailbox is the channel addressed to that
session, and `send`/`mailbox` keep their current shape over it.

## Check

- [ ] `post{channel,from,text}` returns an increasing `seq` per channel, and two
      concurrent posts to one channel get distinct sequence numbers with no lost
      line.
- [ ] `read{channel,since,limit}` returns lines in order from `since`, and a
      `since` beyond the end returns empty rather than an error.
- [ ] Restarting the cartridge returns the same channels, lines and sequence
      numbers from disk.
- [ ] `sessions send{id,from,text}` and `sessions mailbox{id}` keep their current
      behaviour, proven by the existing mailbox test passing unchanged while the
      line is also readable through `read` on that session's channel.
- [ ] A line over the configured length cap is refused with a message naming the
      cap, and the channel is unchanged.
- [ ] A post to an unknown channel creates it; `channels` then lists it with its
      last sequence and activity time.
- [ ] `just check` and `just test` pass.

## Approach

Observed 2026-09-12. The mailbox already exists and is close: `sessions
send{id,from,text}` appends `{"from","to","text","ts"}` as one JSON line to a
lazily created buffer named `mailbox` on the target session and notifies with
that session as the event target; `mailbox{id}` parses the lines back
(`builtin/sessions/main.rs:666`, `:682`). Sessions already persist per session
under `sessions.dir`, so durability is the store's existing property. What is
missing is a name other than a session id, a sequence number, and a read cursor:
`mailbox` returns the whole buffer, which is exactly what a swarm must not do
every turn.

## Spec

Channels are keyed by name. A session-addressed channel keeps the session's
snapshot as its home so a child's messages travel and expire with it; a named
public channel is stored beside the sessions in `sessions.dir` under its own
file. One append path, one parse path, both reached by the mailbox ops and the
channel ops.

A line is `{seq, channel, from, to?, ts, run?, text}`. `seq` is per channel and
monotonic; the store assigns it under the same lock that appends, so it is the
ordering every reader and the log agree on. `text` is capped (default 512 bytes,
configured, refused above it) — the cap is what keeps a channel readable and the
token cost of catching up bounded.

`read` takes `since` and `limit` and is the only way to catch up, so an agent
pays for what it has not seen rather than for the channel's history. Events keep
their current shape: a post notifies with the channel as the target, which is
what lets [[the-agents-chat-through-one-tool]] wake a watcher.
