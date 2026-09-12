---
kind: decision
description: "The agent can inspect, test and extend the program it runs in"
status: accepted
date: "2026-09-12"
---

# The agent can diagnose and extend its runtime

## Choice

The agent can enter debug mode itself to inspect the running program, test its
tools and validate observed effects. It can extend cartridges through the same
visible shell, run checks, reload the changed cartridge and prove the new
behavior. Provide the program context and recipes needed to do that through
discoverable memos and live runtime facts.

## Why

The user's clarification on 2026-09-12 makes debugging an active agent capability.
Understanding the architecture and checking its own actions lets the agent
improve the environment it works in. Logs, tests and context inspection supply
the evidence. The shell is the execution interface and memos explain how to use
and change the current environment.

## Consequences

All relevant program context must be retrievable; the working prompt keeps a
small map and the details needed for the current task. Cartridge extensions
carry usage and test context so later turns can discover them. Runtime facts
must match the loaded revision. Test results preserve failures and uncertainty;
an accepted design does not imply these capabilities are implemented.

Deliver through [[the-agent-can-discover-its-own-program]], [[debug-mode-correlates-a-terminal-turn]] and [[the-agent-can-extend-and-verify-a-cartridge]].
