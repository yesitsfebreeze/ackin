---
kind: work
description: "The agent spawns sub-agents that report back through a mailbox, and the agent view shows and switches between them"
status: open
needs:
  - "[[the-sidebar-slides-over-the-shell]]"
subwork:
  - "[[sub-agent-sessions-record-parent-and-mailbox]]"
  - "[[the-agent-spawns-wakes-and-owns-sub-agents]]"
  - "[[the-panel-switches-to-a-sub-agent]]"
  - "[[the-swarm-talks-on-a-board]]"
  - "[[the-run-is-a-stream-of-typed-events]]"
---

# sub-agents-share-the-terminal

## Outcome

The agent can spawn a sub-agent for a task, send it messages, and be told what
it found. A sub-agent is a session in the same agent process with a parent
recorded on it; it retrieves context through memos and has no implicit shell ownership, because
the visible shell is shared with the user and the root agent. File operations
still go through the visible shell: a child sends an execution request to its
owner, who serializes it and returns attributable readback. No hidden file tool
or per-child PTY is introduced. Messages travel through a mailbox
on the sessions cartridge: the user is a sender like any other, so the composer
and the agent use the same path, and a message to an idle agent wakes it.

The full agent view shows them: a panel lists every live sub-agent with what it is doing
and how long it has been at it, and opening one switches the transcript and
composer to it. Messages appear in both transcripts, naming who sent them.
The gutter's agent column shows the root agent's state; a working sub-agent is
visible there too, so the user knows work is in flight without opening the
full transcript.

## Check

- [ ] Every child listed in `subwork:` is done with recorded evidence.

## Approach

Confirm the agent cartridge runs sessions concurrently before anything else;
if it does not, that is this memo's first commit. Expose `spawn`, `send`, `wait` and `peek` as discoverable session services
invoked through the existing shell and explained by memos. The default model
interface remains `shell` and `memo`. Nothing here spawns a shell, a PTY or a multiplexer
([[the-agent-surface-preserves-the-visible-shell]]). Concurrent edits between sub-agents are a
known hazard and out of scope; a worktree per sub-agent is the answer if it
bites.

Surface amendment, 2026-09-12: collaboration remains wanted. The dependency
on the historically named sidebar work now means the composer/full-transcript
integration, not a left sidebar. Gutter indicators supplement the latest-tool
footer and status; no child output is painted into the foreground editor.
