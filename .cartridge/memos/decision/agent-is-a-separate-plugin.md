---
kind: decision
description: "The coding-agent loop is an independently registered plugin, separate from core and context assembly"
status: accepted
date: "2026-09-09"
uses:
  - usage: "[[read-usage]]"
    when: ["Placing agent behavior or deciding which plugin owns orchestration"]
---

# agent-is-a-separate-plugin

## Decision

The agent is its own Rust process plugin, registered through Lua. It owns model/tool iteration and run control. Context assembly belongs to harness, model transport to router, persistence to sessions, permission decisions to Lua policy, and I/O to tools. The terminal is a client, not a second agent.

## Why

The user explicitly requested a separate agent plugin on 2026-09-09. DeepSeek also separates its agent contract from the loop provider; selected behavior can be ported without adopting Cordis, as recorded in [[deepseek-harness-port-source]].

## Consequences

Replace the loop by editing its Lua wrapper/profile entry without embedding agent behavior in glue runtime or tools. The port map is [[deepseek-plugin-port-map]]. No provider or persistence replacement is needed.
