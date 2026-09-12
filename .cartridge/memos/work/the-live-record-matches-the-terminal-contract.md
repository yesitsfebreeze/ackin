---
kind: work
description: "Active architecture memos describe the current terminal contract"
status: done
level: 10
priority: P2
estimate: 1d
---

# Active architecture memos describe the current terminal contract

## Outcome

A reader can recover the current product and core boundaries from a short authoritative record: intent, environment-specific memos, one visible shell, latest-tool footer, full transcript, and small working context. Earlier designs remain attributable as history without conflicting active instructions.

## Check

- [x] The vision and its active children agree on the latest-tool footer, no empty-tool placeholder, Ctrl+G/Ctrl+F transcript access and keeping the editor surface independent from agent output.
- [x] The record distinguishes core composition/transport from product services and documents the rationale for each default dependency. Review the proxy-specific launch branch in core/main.rs and explicitly retain it as CLI policy or create a bounded extraction item; do not relocate generic transport just to shrink the core.
- [x] Current reproduction commands use existing crate/file names; historical evidence stays dated. Replace stale totals and completion claims with observable checks, and resolve conflicting active decisions through the decision supersession convention.

## Approach

Observed 2026-09-12: `the-vision` still says replies enter shell scrollback and cites a fixed total of 43 memos. `lua-only-core-boundary` retains historical glue/core/plugin command names. Preserve the historical results, but make current contracts and commands unambiguous.

Audit: [[runtime-audit-2026-09-12]].

## Result

2026-09-12 — reconciled 17 record files through validated native memo writes,
all saved without warnings and read back with revisions. The accepted terminal
decision preserves the latest-tool/footer/full-transcript contract while grid,
input encoding and sub-agent work remain open. Three explicit supersession edges
preserve the historical decisions. Current core/CLI boundaries and each default
service role are recorded; proxy launch remains narrow CLI policy. Older `glue`
commands and scrollback checks are labelled historical rather than erased.

Validation: composed work graph has no open/active off-axis memo, missing work
edge or cycle; all changed open work has Outcome and Check. `git diff --check`
passed. This was a record change: no runtime tests were rerun or new product
capabilities claimed. Coordinator owns the serial record commit.
