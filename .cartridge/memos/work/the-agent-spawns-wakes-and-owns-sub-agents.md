---
kind: work
description: "The agent cartridge spawns, wakes, waits on and owns sub-agents, keeping the visible shell to the owner"
status: open
needs:
  - "[[sub-agent-sessions-record-parent-and-mailbox]]"
---

# the-agent-spawns-wakes-and-owns-sub-agents

## Outcome

The agent cartridge serves sub-agents — sessions in the same agent process with
a parent recorded on them — concurrently, and exposes `spawn`, `send`, `wait`
and `peek` as discoverable session services invoked through the existing shell
and explained by memos. Sending a message into an idle sub-agent's mailbox
wakes a new run whose prompt carries the parked message. The visible shell
stays owned: a sub-agent's file-operation request routes to its owner, who
serializes it and returns attributable readback; an unowned execution is
refused and the user's shell is untouched. The parent records what a child
reports into its own transcript naming the child.

## Check

- [ ] A test spawns a sub-agent session, starts its run, sends it a message,
      wakes a new run with the parked message, and reads the child's report in
      the parent's transcript.
- [ ] Two sub-agent sessions run at once and both reach a terminal state without
      either blocking the other.
- [ ] A sub-agent execution request naming no owner is refused, and the probe
      records that no tool reached the user's shell.
- [ ] A sub-agent's execution request routed to its owner returns attributable
      readback visible in the child's transcript.
