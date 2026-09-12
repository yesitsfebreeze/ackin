---
kind: decision
description: "The terminal stays the product; a desktop or browser surface attaches later over the same core, once the terminal work is done and proven"
status: accepted
date: "2026-09-12"
uses:
  - usage: "[[read-usage]]"
    when: ["Choosing a frontend, scoping UI work, or deciding where surface effort goes"]
---

# desktop-is-a-later-client-over-the-same-core

The wrapper's home is the terminal: the agent is invisible, the user is in
their own shell, and interactive programs keep their screen. A desktop or
browser application is not a rewrite and not a now-goal — it is an additional
client over the same daemon and socket, attached once all the terminal work
([[the-terminal-is-drawn-from-pty]], [[the-palette-and-exit]],
[[copy-mode-interacts-with-the-text]], [[context-corrections-carry-their-context]])
is done and proven. It would consume the same grid, marks and session services
the TUI consumes, over a fast local or network link.

The deeper shape this points at: the system is an orchestrator that does real
work, and can grow into something operating-system-like if that is ever
wanted. The cartridge/socket core is what makes that malleable; surface
decisions must never bake the core into one frontend.

Consequence: no GUI work is scheduled until the terminal contract ships.
Requests for richer surfaces land on the terminal memos first.
