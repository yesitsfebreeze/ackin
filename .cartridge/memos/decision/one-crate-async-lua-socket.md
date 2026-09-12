---
kind: decision
description: "glue is one Rust crate of plain modules implementing the paper's runtime on tokio, scripted by Lua 5.4 plugins, served on a Unix socket; the paper's framework is read, never depended on"
status: superseded
date: "2026-09-09"
uses:
  - usage: "[[read-usage]]"
    when: ["asking what glue is built from, why one crate, or where the runtime, plugins and wire live"]
---

## Superseded on 2026-09-12

[[core-composes-and-the-cli-selects-services]] carries the current crate names,
paths and commands. The original naming and rationale below are historical.

# one-crate-async-lua-socket

## Decision
`glue` is a single package: `src/runtime.rs` (context, effects, coeffects,
access, events), `src/fiber.rs` (lifecycle), `src/lua.rs` and
`src/context.rs` (Lua host and the plugin `ctx`), `src/loader.rs`
(`glue.lua` entries, file watch), `src/socket.rs` (JSON lines on a Unix
socket), `src/main.rs` (`glue daemon | send | tail | reload | status`). Async
and multithreaded on tokio; plugins are Lua 5.4 through mlua, vendored, with
`send`; the framework name the paper uses appears nowhere in source. This
supersedes [[cordis-is-reference-only]] only in that the ideas are now
implemented here; the framework itself is still not a dependency.

## Why
The owner wants one base layer a later UI runs in the background and talks
to by events, not a second application. Three crates in `src/` were
overhead for one binary; plain modules named for what they do read faster.
Async because this layer is the basis of a larger system and must not
block; Lua because plugins must be writable without a Rust build; a socket
because more than one client will attach.

## Consequences
Tests are named for what they hold: `src/tests/lifecycle.rs` (effects,
dependencies, events), `src/tests/plugins.rs` (yields, access, loader),
`src/tests/socket.rs` (round trip). `just check` and `just test` are the gates;
see [[memos]]. The teardown order follows the paper, see
[[cordis-teardown-ordering-gap]].
