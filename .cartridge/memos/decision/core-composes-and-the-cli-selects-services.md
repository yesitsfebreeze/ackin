---
kind: decision
description: The core owns composition and transport; cartridge behavior and narrow CLI launch policy remain separate
status: accepted
date: "2026-09-12"
supersedes: "[[one-crate-async-lua-socket]]"
---

## Choice

`zirkle` is one Rust host crate under `core/`. Runtime/fiber own generic composition,
effects and lifecycle. Lua/context/loader bind cartridges to it; cartridge/sdk
own the process protocol and socket owns local transport. Agent orchestration,
context projection, model routing, persistence, tools and rendering are cartridge
behavior. Do not extract generic transport merely to reduce the core's size.

Retain the `core/main.rs` launch branch as narrow CLI policy: select the proxy
profile, supply a per-launch proxy key, invoke the proxy's launch service and
keep its host alive until the child exits. `builtin/proxy` and `builtin/router`
retain provider-specific launch behavior and request policy. Expanding this
branch into model/tool orchestration would cross the boundary.

## Why

This preserves [[lua-only-core-boundary]] and [[tui-and-tools-are-cartridges]].
Source inspection on 2026-09-12 shows `Host::run_then("proxy", request, spawn)`
in the launch branch and generic lifecycle/transport in the library. Moving the
small CLI entrypoint now would create a migration without removing a second
implementation. The older accepted memo used the retired name `glue` and `src/`
paths; its architectural reasons remain historical evidence.

## Consequences

Current commands are `zirkle list`, `zirkle run <service> <json>` and `zirkle call
<service> <json>`. Gates are `just check` and `just test`. Focused host checks use
`cargo test -p zirkle --lib tests::process`, `tests::lifecycle`, `tests::cartridges`
and `tests::socket`, not the retired `glue` crate or `core/plugin.rs`.

The default profile's dependencies have these roles, inspected on 2026-09-12:

| Entry | Purpose and reason to load it |
| --- | --- |
| memo | Validated knowledge record, usage resolution and system memo composition. |
| sessions | Durable turns, buffers and session state needed by the agent and UI. |
| router | Model selection, credentials and request transport. |
| policy | Tool authorization independently of model and shell execution. |
| harness | Small task context, prompt projection and compaction. |
| agent | Model/tool loop and run control. |
| pty | The visible shell, environment facts and interactive `shell` readback. |
| ui | Renderer, composer, status and transcript; depends on services it presents. |
| clock | Injectable UTC date currently required by harness; simplification is [[context-date-needs-no-clock-cartridge]]. |
| memory | Separate semantic engine currently enabled; proxy recall consumes it, while native startup without it is [[terminal-profile-starts-only-needed-services]]. |

The host's in-memory reload banks, the native memo record, the memory engine and
compacted conversation context have different responsibilities. Removing an
unused startup dependency does not authorize deleting their persisted data.
The default agent tools remain `shell` and `memo`; other services are reachable
through the shell when explicitly exposed and documented.
