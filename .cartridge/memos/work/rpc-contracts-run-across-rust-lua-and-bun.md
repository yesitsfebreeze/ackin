---
kind: work
description: "The language boundaries share executable RPC contract cases"
status: active
owner: "codex-work-2026-09-12/rpc-contracts"
level: 10
priority: P1
estimate: 2d
---

# The language boundaries share executable RPC contract cases

## Outcome

Rust SDK processes, Lua services and the Bun UI obey one documented host protocol, verified through shared behavioral cases. Keep transport and lifecycle infrastructure in the core; reuse existing fixtures instead of creating another runtime.

## Check

- [ ] A real-host suite exercises Rust-to-Lua and host-to-Bun round-trips for null, false, empty arrays/objects, Unicode, errors and concurrent bidirectional calls with overlapping numeric IDs.
- [ ] Cases cover apply failure, reload/dispose, EOF with pending calls and late replies; each participant has an explicit applicability matrix.
- [ ] Tests distinguish a supported timeout/cancellation difference from a protocol violation and run under the normal gate.

## Approach

Observed 2026-09-12: `core/tests/process.rs` already checks Rust/Lua ID namespaces, EOF, metadata and dropped waiters. `builtin/ui/tests/wire.test.ts` has one test for out-of-order replies and disconnect. Extend that foundation; the missing result is shared coverage, not a claim that no integration tests exist.

Audit: [[runtime-audit-2026-09-12]].
