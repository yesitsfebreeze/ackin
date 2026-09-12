---
kind: work
description: "Lua process wrappers with effective dependencies and watches"
status: done
needs:
  - "[[process-plugins-register-through-lua-rpc-roundtrip]]"
uses:
  - usage: "[[read-usage]]"
    when: ["Registering a Rust process through a Lua wrapper, declaring extra injected keys, or preserving config and reload behavior"]
---

# process-plugins-register-through-lua-lua-registration

## Do

Implement `glue.process(command, {inject={...}})` as a Lua-visible descriptor for the existing process component, not a second protocol implementation. A wrapper returns that descriptor. Merge explicit extra injected keys with hello.inject before constructing the fiber; exact keys only, no tool.* wildcard. Entry configuration still arrives at apply. Add an optional entry inject list which augments component declarations before fiber construction, so one profile-local tools table can populate both agent config.tools and its injections. Deduplicate static/extra keys and expose effective declarations in glue list. All registered plugins enter by path; reject legacy cmd entries, including path+cmd, with migration guidance.

First fix the current process-to-host reply-direction defect: host handling of child call/meta requests must send reply frames to the child, not resolve host-local pending IDs. Preserve wire frame shapes and existing SDK callers. This is a dependency of every new Rust plugin using host.call. Own plugin wrapper migration and bidirectional link lifecycle. Keep core/plugin.rs and glue::sdk; literal transport extraction is not required by the recorded registration decision. No product-specific tool execution enters the core.

This memo owns only the bounded unit below; linked prerequisites own their implementations. Agent separation follows [[agent-is-a-separate-plugin]].

## Spec

Planning status: proposed specification based on source inspection. No implementation probe or code tests have run. Keep every acceptance box unchecked until verified. File footprint: `core/loader.rs`, `core/lua.rs`, `core/plugin.rs`, `core/context.rs`, `core/tests/`, `plugins/sessions.lua`, `plugins/router.lua`, `.zirkle/default/init.lua`, `CLAUDE.md`.

Expose the process descriptor helper while retaining normal Lua component tables. Resolve hello declarations once per load; merge descriptor extra injections and entry.inject before component creation. Reject invalid/duplicate-conflicting declarations, nonexistent commands and legacy cmd entries explicitly. Do not let an apply-time config field silently grant a dependency. Preserve exact-key runtime access restrictions and config overrides. The wrapper can contain ordinary Lua composition; do not require only one syntactic form.

Migrate sessions/router default entries to Lua wrappers. Track both wrapper source and declared executable paths, adding watches when a profile reload introduces a binary in a new directory. Replacing a binary or wrapper disposes the old fiber exactly once, starts a replacement and maintains dependency ordering. A disabled entry does not spawn its process. Existing wire serialization stays unchanged. Update only the affected registration paragraph in CLAUDE.md and process module docs; do not rewrite unrelated working instructions.

## Check

- [x] The migrated default profile loads sessions/router through Lua; glue list reports original provides plus effective merged injections.
- [x] Config overrides reach the process unchanged; disabled entries do not spawn; undeclared dependencies still fail.
- [x] Legacy cmd-only and mixed path+cmd entries fail with an actionable wrapper example.
- [x] Changing wrapper or binary restarts once; a newly introduced executable directory is watched after profile reload.
- [x] Adding one extra tool key through entry.inject is visible before fiber construction and callable by the process without rebuilding it.

Verification commands (future implementation gate; not run during planning):

```sh
set -eu
cargo test -p glue --lib tests::process
cargo test -p glue --lib tests::plugins
test -f plugins/sessions.lua
test -f plugins/router.lua
cargo clippy -p glue --all-targets -- -D warnings
```

Landed in `snapshot/plugin-workspace`: `8a283b9`, `2023bdc`, `ebe7a11`, `c09bedb`. All five checks passed, including live executable-directory reload, disabled no-spawn, merged injections, config, legacy errors, and migrated default manifest. Spec block: process 10/10, plugins 4/4, wrapper existence and Clippy passed. Combined `just all`: 28/28, fmt/Clippy/docs passed. SDK fixture source moved under `core/tests/fixtures` following user feedback. Lane closed after owner release, generated Cargo output cleanup, and ancestry/cleanliness checks.
