---
kind: work
description: "Simplify terminal dependencies and make the runtime reproducibly verifiable"
status: open
level: 9
subwork:
  - "[[context-date-needs-no-clock-cartridge]]"
  - "[[terminal-profile-starts-only-needed-services]]"
  - "[[rpc-contracts-run-across-rust-lua-and-bun]]"
  - "[[fresh-checkouts-can-run-the-gates]]"
  - "[[ci-proves-the-supported-terminal-matrix]]"
  - "[[the-agent-can-discover-its-own-program]]"
  - "[[debug-mode-correlates-a-terminal-turn]]"
  - "[[the-agent-can-extend-and-verify-a-cartridge]]"
  - "[[rolling-context-retains-decision-evidence]]"
  - "[[the-live-record-matches-the-terminal-contract]]"
  - "[[cartridges-prove-themselves-at-registration]]"
  - "[[lanes-do-not-poison-each-others-builds]]"
  - "[[the-reload-test-is-not-flaky]]"
---

# A small, provable terminal runtime

## Outcome

The terminal-native agent has only justified runtime dependencies, a compact
current design record, and reproducible evidence for language boundaries,
agent-driven debugging, cartridge self-extension and context quality. This is the bounded improvement set from
[[runtime-audit-2026-09-12]], not an open-ended rewrite of the core.

P1 is the first implementation group; P2 is follow-up cleanup. Dependencies in
`needs` define ordering; the rows are otherwise independently actionable.

| Priority | Work | Estimate |
| --- | --- | --- |
| P2 | [[context-date-needs-no-clock-cartridge]] | 4h |
| P1 | [[terminal-profile-starts-only-needed-services]] | 1d |
| P1 | [[rpc-contracts-run-across-rust-lua-and-bun]] | 2d |
| P1 | [[fresh-checkouts-can-run-the-gates]] | 1d |
| P1 | [[ci-proves-the-supported-terminal-matrix]] | 2d |
| P1 | [[the-agent-can-discover-its-own-program]] | 1d |
| P1 | [[debug-mode-correlates-a-terminal-turn]] | 2d |
| P1 | [[the-agent-can-extend-and-verify-a-cartridge]] | 2d |
| P1 | [[rolling-context-retains-decision-evidence]] | 1d |
| P2 | [[the-live-record-matches-the-terminal-contract]] | 1d |
| P1 | [[cartridges-prove-themselves-at-registration]] | 2d |

Clarification: [[the-agent-can-diagnose-and-extend-its-runtime]] makes program discovery → debugging →
extension a testable agent workflow, with diagnostics as supporting evidence.

## Check

- [ ] Every child has its acceptance evidence and is done.
- [ ] The audit is updated with the resulting dependency graph, gate results,
      debug invocation and remaining explicit limits.
