---
kind: work
description: "A dispatched agent asks the graph for the best tool and the best way, with no per-session setup"
status: active
owner: "sys-work-2026-09-12/analyst-agents-query-the-tool-graph"
level: 10
priority: P1
needs: ["[[tool-dispatch-and-routines-are-graph-nodes]]"]
---

# agents-query-the-tool-graph

## Do

A dispatched agent queries the graph through a tool key — a `tool.graph`
surface or a graph operation on an existing service, whichever the engine and
the tool envelope settle on — with no per-session setup. A query like "best
tool for X" or "how we do Y" answers from the graph: ranked entries with
descriptions, the best way first, each carrying why it ranked (the match and
the observed use behind it). One ranked surface, consulted the same way by
every dispatched agent.
