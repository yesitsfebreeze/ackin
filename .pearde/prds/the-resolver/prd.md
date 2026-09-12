---
state: open
origin: requested
priority: 95
complexity: 0
blast-radius:
needs:
  - the-ledger
---

# The resolver

The mechanism, and the reason this system is not a supervisor.

You name a tool. The binary reads [[the-ledger]], walks that tool's dependency
chain to its far end, and spawns the **first** program the chain requires — not
the tool that was asked for. That program comes up and re-enters the binary for
the next tool along the chain. That one comes up and re-enters it again. The
chain assembles itself from the bottom, one re-entry per link, until the named
tool is running at the top of a tree that was resolved entirely at runtime, out
of whatever the ledger held at the moment of the ask.

The binary is therefore invoked once per node, and each invocation has exactly
one job: resolve a single step and hand off. Nothing sits above the tree owning
it. There is no long-lived coordinator to lose state, to restart, or to become
the thing every cartridge waits on.

The property that must hold, and must be demonstrable rather than asserted:
because a dependency *launches* its dependent, the process tree and the
dependency tree are the same tree. Nothing maintains that correspondence, so it
cannot drift. A node exits and everything that needed it goes with it — teardown
is correctly ordered for free, and uninstalling is the absence of a launch.

Pointers: `deps()` in `src/main.rs` already performs this walk for display, and
`src/loader.rs` already resolves an `inject` key to its providing cartridge. The
walk exists; this PRD makes it the launch path instead of a report.

Must not change: a cycle in the chain is still detected and still refused —
`deps()` tracks a stack for exactly this and the launch path inherits the duty.

At the end, starting any tool brings its whole chain up from the bottom, and
`ps` shows the dependency tree.
