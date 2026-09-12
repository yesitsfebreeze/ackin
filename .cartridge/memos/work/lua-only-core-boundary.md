---
kind: work
description: "Verify wrapped processes obey ordinary composition"
status: done
needs:
  - "[[process-plugins-register-through-lua-lua-registration]]"
uses:
  - usage: "[[read-usage]]"
    when: ["Deciding what Lua-only registration means or testing whether wrapped processes obey isolation, metadata interception and lifecycle rules"]
---

## Current boundary and reproduction — 2026-09-12

Follow [[core-composes-and-the-cli-selects-services]]. Runtime/fiber, lifecycle,
Lua registration, SDK process transport and sockets stay in the generic host;
agent, tools, policy, context and routing remain cartridges. The narrow proxy
launch branch is retained CLI policy, not model orchestration.

Current names are `core/cartridge.rs`, `zirkle::sdk` and crate `zirkle`. Run:

```sh
cargo test -p zirkle --lib tests::process
cargo test -p zirkle --lib tests::cartridges
cargo test -p zirkle --lib tests::lifecycle
cargo test -p zirkle --lib tests::socket
```

The original Do/Spec, `glue` commands and 32-test result below are dated delivery
history. They do not prescribe current filenames or imply another probe is due.

# lua-only-core-boundary

## Do

This work item is a boundary regression gate, not another implementation of process wrapping. The existing board decision explicitly retains core/plugin.rs and glue::sdk: Lua-only means plugin registration/composition, not removal of Rust APIs or the local client socket. Runtime/fiber remain the composability model; process/socket code is host infrastructure. Product behavior (agent orchestration, tools, policy, context and model routing) stays in separate plugins.

The previous placeholder's proposed transport extraction duplicated the wrapper item and contradicted that recorded scope. Do not move crates merely to satisfy that placeholder. Prove wrappers compose through context isolation/interception and ensure a profile does not require loader branches for individual plugin names. Literal transport extraction would require a separately approved compatibility migration, not an implicit part of these specs.

This memo owns only the bounded unit below; linked prerequisites own their implementations. Agent separation follows [[agent-is-a-separate-plugin]].

## Spec

Planning status: proposed specification based on source inspection. No implementation probe or code tests have run. Keep every acceptance box unchecked until verified. File footprint: `core/tests/`, `core/plugin.rs`.

Add focused composition regression tests to the existing glue library tests. A wrapped process uses an alternate provider in an isolated context. Interception changes metadata, not callable service values: query it through Host.meta and verify the intercepted metadata reaches the process. Removing/restoring dependencies follows normal fiber lifecycle. Test local manifest loading/glue list independently from socket calls; there is no socket list request. Do not invent callable interception, key-remapping APIs or another wire protocol. Keep core transport infrastructure and glue::sdk unchanged.

## Check

- [x] An isolated wrapped process sees its alternate service provider, not the global provider.
- [x] Intercepted metadata reaches the child through Host.meta; absent metadata remains null.
- [x] Dependency removal disposes the process and restoration restarts it without leaked effects.
- [x] Local manifest/list behavior and existing socket call behavior remain compatible.

```sh
set -eu
cargo test -p glue --lib tests::plugins
cargo test -p glue --lib tests::lifecycle
cargo test -p glue --lib tests::socket
```

## Result

2026-09-09 22:05 — boundary tests landed on work/core-boundary (b90f229, e9273fa); 4/4 checks, 32 core tests + sh block green
