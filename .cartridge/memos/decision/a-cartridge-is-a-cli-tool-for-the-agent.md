---
kind: decision
description: "Cartridges are the agent's CLI tools: the shipped surface is the minimal agentic loop, and every persona, memo set and tool rides in a cartridge the profile can load or unload at runtime"
status: accepted
date: "2026-09-12"
---

# a cartridge is a CLI tool for the agent

The builtin cartridges are git-tracked and the profile can load or unload
any of them at runtime, from the user's directory or from the local
repository. A cartridge is to the agent what a CLI tool is to a shell: we
build whatever the work needs inside one, and the composed system extends
by enabling it, not by editing the core.

The shipped memos and base information hold the goal: the most minimal
surface we can think of that runs a working agentic loop and does real
work. Everything beyond that loop — tooling, memos, personas — loads via
cartridges. A persona ships with its persona cartridge and its persona
memos inside; enabling it makes them part of the record, disabling it
removes them. The landscape graphs exactly the enabled cartridges, so
[[the-landscape-owns-the-graph]] loads only what a job needs and the
profile stays the single switch.

See [[tui-and-tools-are-cartridges]] and
[[the-agent-can-diagnose-and-extend-its-runtime]].