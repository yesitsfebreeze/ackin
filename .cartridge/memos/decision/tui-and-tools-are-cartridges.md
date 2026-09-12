---
kind: decision
description: "TUI and repository tooling are cartridges invoked by a generic foreground wrapper"
status: accepted
date: "2026-09-10"
uses:
  - usage: "[[read-usage]]"
    when: ["adding behavior to core or invoking interactive and repository services"]
---

# tui-and-tools-are-cartridges

## Decision

The user requires a deliberately slim core: behavior belongs to cartridges and core wraps plugin loading, composition, transport and lifecycle. The UI provides `ui` through the SDK and injects `agent`, `sessions`, and `buffers`. Repository build/bundle/lane operations provide `tools` from `builtin/tools/`, not a second core executable.

## Why

A standalone terminal client and repository commands in core bypassed the extension model. Keeping their behavior in cartridges makes both lifecycle-managed and replaceable through manifests.

## Consequences

`zirkle run <service> <json>` loads a profile, waits for the requested provider, invokes it, then disposes all instances on success or failure. `just run` uses the `ui` service. The TUI owns terminal foreground acquisition, rendering, input, approval handling, cancellation, and restoration; protocol stdin/stdout remain separate. The tools-only profile avoids loading agent services for repository operations. Bundles ship the core wrapper at the root and the TUI/tools executables inside their cartridge folders.

Naming correction, 2026-09-12: the earlier `glue`/`tui` names are now `zirkle`/`ui`.
The boundary is clarified by [[core-composes-and-the-cli-selects-services]];
the UI surface follows [[the-agent-surface-preserves-the-visible-shell]].
