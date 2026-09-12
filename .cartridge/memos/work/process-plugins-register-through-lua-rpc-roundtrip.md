---
kind: work
description: "Bidirectional RPC replies and pending-request cleanup"
status: done
uses:
  - usage: "[[read-usage]]"
    when: ["A process plugin hangs calling an injected service, receives the wrong RPC reply, or leaves a request pending after disconnect"]
---

# process-plugins-register-through-lua-rpc-roundtrip

## Do

Implement `glue.process(command, {inject={...}})` as a Lua-visible descriptor for the existing process component, not a second protocol implementation. A wrapper returns that descriptor. Merge explicit extra injected keys with hello.inject before constructing the fiber; exact keys only, no tool.* wildcard. Entry configuration still arrives at apply. Add an optional entry inject list which augments component declarations before fiber construction, so one profile-local tools table can populate both agent config.tools and its injections. Deduplicate static/extra keys and expose effective declarations in glue list. All registered plugins enter by path; reject legacy cmd entries, including path+cmd, with migration guidance.

First fix the current process-to-host reply-direction defect: host handling of child call/meta requests must send reply frames to the child, not resolve host-local pending IDs. Preserve wire frame shapes and existing SDK callers. This is a dependency of every new Rust plugin using host.call. Own plugin wrapper migration and bidirectional link lifecycle. Keep core/plugin.rs and glue::sdk; literal transport extraction is not required by the recorded registration decision. No product-specific tool execution enters the core.

This memo owns only the bounded unit below; linked prerequisites own their implementations. Agent separation follows [[agent-is-a-separate-plugin]].

## Spec

Planning status: proposed specification based on source inspection. No implementation probe or code tests have run. Keep every acceptance box unchecked until verified. File footprint: `core/plugin.rs`, `core/sdk.rs`, `core/tests/process.rs`.

Trace Link.request, Link.answer and every call/meta reply producer. Send child-originated responses back over the wire with their original IDs; use local pending resolution only for received reply frames. Preserve success/error envelopes. Make pending registration cancellation-safe: dropping a waiting request removes only that local pending entry; it does not pretend the remote handler was cancelled. Peer EOF or disposal resolves remaining waiters with an error instead of hanging. Keep separate peer-local ID namespaces; overlapping numeric IDs in opposite directions are legal.

Use a real SDK child fixture in existing glue library process tests. Exercise a child calling an injected Lua service, querying metadata and returning the result to a concurrent host request. No live daemon or provider credentials.

## Check

- [x] A real child can call a Lua-provided service and return its data to its caller; Lua errors propagate as error replies.
- [x] Child meta requests return metadata over the wire, including null when metadata is absent.
- [x] Concurrent calls using identical numeric IDs in opposite directions return to the correct waiter.
- [x] Peer EOF/disposal releases pending calls; dropping a wait future removes its pending entry and ignores late replies.
- [x] Existing process call/event/disposal tests retain their wire behavior.

Verification commands (future implementation gate; not run during planning):

```sh
set -eu
cargo test -p glue --lib tests::process
test -f core/tests/process.rs
cargo clippy -p glue --all-targets -- -D warnings
```

Implemented and landed in `snapshot/plugin-workspace` as `3d539a7` and `4ed0e08`. Real SDK tests cover every Check and reproduce the original reply-direction failure under a negative control. Focused process tests: 5 passed; `just check` passed; combined `just all`: 23 passed, 0 failed, doc tests passed. Lane closed after ancestry, cleanliness, and owner-release checks.
