---
kind: note
description: "Observed runtime boundaries, verification evidence and bounded improvements"
date: "2026-09-12"
---

# Runtime audit — 2026-09-12

Scope: local source and gate evidence from this session, working tree based on
`71a10a8` with uncommitted changes. This is a boundary review, not line-by-line
certification or measured coverage. Work: [[runtime-stays-small-and-provable]].

## What to crystallize

The user's terminal is the execution surface. The agent expresses intent,
retrieves environment-specific recipes from native memos, acts through `shell`,
and reads back the observed effect. The footer shows only the latest tool;
the transcript preserves the whole account outside a running editor.

Working context preserves goal → reason, decision → evidence, path → finding →
effect, and action → confirmed result. Keep corrections, uncertainty and next
steps. Retrieve details when needed; retain the journal as evidence. Capability
recipes belong in memos. A runtime service earns its place through an actual
consumer or an independent lifecycle; a tiny helper need not be a cartridge.

## Debugger clarification — 2026-09-12

The user clarified that the agent must be able to enter debug mode itself,
test tools, inspect results, and extend cartridges using retrievable knowledge
of the program. This expands the original logging-focused work into a
diagnosis and self-extension loop. [[the-agent-can-diagnose-and-extend-its-runtime]] records the choice;
[[the-agent-can-discover-its-own-program]], [[debug-mode-correlates-a-terminal-turn]] and [[the-agent-can-extend-and-verify-a-cartridge]] carry its checks.
These are planned outcomes; the observations below describe the audited implementation.

## Observations

| Area | Evidence and interpretation |
| --- | --- |
| Core | `core/README.md`, runtime/fiber/context/loader and `core/tests/process.rs`: generic composition, transactional reload and process transport have meaningful tests. Keeping this infrastructure in the core agrees with [[lua-only-core-boundary]]. Proxy launch configuration in `core/main.rs` is retained as narrow CLI policy by [[core-composes-and-the-cli-selects-services]]. |
| Clock | Six-line `builtin/clock/init.lua` supplies a UTC date. The harness requires that service. Simplification candidate; no extra process to remove. |
| Memory | Both profiles enable the separate engine. `builtin/proxy/service.rs::recall` consumes it; no native agent/harness consumer was found. Prove native startup without it while retaining proxy recall. Host reload snapshots and rolling context are separate mechanisms. |
| Languages | Rust tests execute real Lua composition and SDK children; Bun tests exercise UI and wire; Python tests exercise terminal processes. Shared protocol cases across Rust/Lua/Bun remain thin, especially Bun error/lifecycle paths. TypeScript checking already includes tests via `builtin/ui/tsconfig.json`. |
| Gates | `just check` runs formatting, warnings-denied clippy and TypeScript checking; `just test` runs memory/workspace tests, doctests, Bun and Python suites. Python PTY test files run in separate processes. |
| Reproducibility | `builtin/memory` is ignored but required by both gates. Lanes borrow a trunk symlink. No tracked `.github` workflows were found; external CI and branch protection were not inspected. Optional shell/editor Rust tests can return early when executables are absent. |
| Debugging | `/context` and Ctrl+O inspect context; status/tail/list and stderr diagnostics exist. `just run` overwrites `.zirkle/logs/ui.log`. No unified debug switch or correlated host/Lua/UI trace was found. The memory process separately supports RUST_LOG. |
| Compaction | `working.rs`, `working_tests.rs` and the process test cover bounds, refresh, persistence and failures. The process fixture supplies a canned summary; semantic retention across repeated model summaries is unmeasured. |
| Record | Initial audit found scrollback/sidebar-only instructions, footer removal and stale glue commands. [[the-live-record-matches-the-terminal-contract]] now reconciles them through explicit supersession and dated historical evidence; compatible grid and collaboration work remains open. |

## Verification observed in this session

- `just check` passed (Rust and memory formatting/clippy, TypeScript).
- Workspace nextest: 221 passed, zero skipped after correcting the stale
  compaction fixture budget. Workspace doctests passed.
- Separate memory workspace: 1,337 passed, 17 skipped; its doctests passed.
- Bun UI: 20 passed. Python: tools 7, development 1, router 13, shell 6, UI 1
  passed. These are local counts, not a supported-platform coverage claim.
- The initial full test invocation stopped at the stale compaction fixture;
  the corrected workspace suite and remaining gate steps passed separately.

Local ephemeral logs: `/tmp/zirkle-shell-check.log`,
`/tmp/zirkle-shell-full-tests.log`, `/tmp/zirkle-shell-workspace-tests.log`,
`/tmp/zirkle-shell-doc-tests.log`. The Python completion was observed in the
session output. No second full code-gate run is needed for this memo-only audit;
new memo writes and their board links are validated separately.
