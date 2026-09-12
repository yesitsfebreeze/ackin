---
kind: work
description: "The full agent view lists live sub-agents and switches transcript and composer to one; a working sub-agent is visible in the gutter"
status: open
needs:
  - "[[the-sidebar-slides-over-the-shell]]"
  - "[[the-agent-spawns-wakes-and-owns-sub-agents]]"
---

# the-panel-switches-to-a-sub-agent

## Outcome

The full agent view shows the live sub-agents: a panel lists every live
sub-agent with what it is doing and how long it has been at it. Opening one
switches the transcript and composer to it; closing returns to the root agent,
and the editor keeps its size and cursor. Messages appear in both the sender's
and receiver's transcripts, naming who sent them. The gutter's agent column
shows the root agent's state, and a working sub-agent is visible there too, so
the user knows work is in flight without opening the full transcript. No child
output is painted into the foreground editor. This depends on the composer and
full-transcript integration, not on a left sidebar.

## Check

- [ ] A real UI run lists a live sub-agent in the panel; opening it swaps the
      transcript and the next message goes to it; closing returns to the root
      agent with the editor size and cursor unchanged.
- [ ] A message from the user or the root agent to a sub-agent appears in both
      transcripts naming who sent it.
- [ ] A working sub-agent shows in the gutter's agent column without opening
      the full transcript.
- [ ] Two live sub-agents appear in the panel at once, each keeping its own
      transcript.
