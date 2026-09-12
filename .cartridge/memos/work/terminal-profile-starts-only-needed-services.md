---
kind: work
description: "The native terminal runs without the separate recall engine"
status: active
owner: "codex-work-2026-09-12/coordinator"
level: 10
priority: P1
estimate: 1d
---

# The native terminal runs without the separate recall engine

## Outcome

The default native terminal starts with only the services it uses. Durable semantic recall remains available to the proxy profile, which consumes it. Native memos, rolling context and host reload snapshots keep their distinct ownership and stored data.

## Check

- [ ] The default profile starts and completes a shell/memo agent turn with the memory engine unavailable, without starting or building that engine.
- [ ] Proxy recall still returns a seeded memory through its explicitly enabled memory service.
- [ ] Profile dependency tests and documentation distinguish native memos, rolling summaries, host snapshots and optional semantic recall; existing memory data is untouched.

## Approach

Observed 2026-09-12: Both profiles enable `memory`; the product consumer found is `builtin/proxy/service.rs::recall`, declared by `builtin/proxy/main.rs`. No native agent/harness consumer was found. Prove the complete dependency graph before changing startup.

Audit: [[runtime-audit-2026-09-12]].

## Spec

The probe is implemented in `e508e70` and `968927c`, integrated by fast-forward.
Default init/config and native startup recipes omit memory. Foreground builds
select enabled and live nested cartridge sources. Proxy recall is explicit and
has `just proxy-smoke`. Remaining integration gate: `just all`.

## Verification checkpoint

Profile 10, source/nested lifecycle 2, live reload 1, proxy 12 and real optional
recall smoke 1 passed. With memory directory and target binary unavailable,
`just build` plus the exact native shell/memo turn passed. Formatting, workspace
Clippy, TypeScript and UI 20 passed. Full integrated gates are running; ownership
has returned to the coordinator. No remaining implementation scope is known.
