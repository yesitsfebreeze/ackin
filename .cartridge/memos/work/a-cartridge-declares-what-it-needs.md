---
kind: work
description: "A cartridge's manifest states the keys it provides and needs, so the host resolves a chain from data instead of a hand-wired profile"
status: active
owner: "sys-38/implementer-a-cartridge-declares-what-it-needs"
level: 10
estimate: 1d
---

# a-cartridge-declares-what-it-needs

## Outcome

A cartridge folder states its own contract: the keys it provides, the keys it
needs, and for each need whether it is shared with whoever composes it or
private to this instance. The host resolves each need to a providing cartridge
by reading manifests, without evaluating Lua and without spawning a process,
and reports an unresolved or ambiguous need as one line naming the need, the
cartridge and the candidates. A profile entry still overrides any resolution,
so composition by hand stays possible where it is wanted.

Scope is the manifest contract and its resolution. Who starts the resolved
chain, and when, belongs to [[a-key-starts-its-provider-on-demand]].

## Check

- [ ] A fixture cartridge declaring `needs` loads with no `inject` in the
      profile entry and reaches its provider's service.
- [ ] Two fixture cartridges providing the same key make a run fail with a
      message naming the key and both candidates, and a profile entry naming
      one of them resolves it.
- [ ] A declaration read for a process cartridge spawns no process: the test
      asserts the executable's invocation count is zero while declarations are
      resolved.
- [ ] A manifest whose declared `provide` disagrees with what the cartridge
      registers at apply fails that entry with a message naming both sets, and
      leaves other entries serving.
- [ ] A need declared private gets its own realm: two instances of the same
      parent cartridge each reach a distinct provider instance, proven by
      distinct values from the same key.
- [ ] `just check` and `just test` pass.

## Approach

Observed 2026-09-12. `cartridge.json` carries `name`, `entry`, `binary`, `ui`,
`selftest`, `integration` and `source` (`core/loader.rs:73`) — no keys. A
cartridge's `provide` is learned only by running it: the Lua entry declares
`inject`, and `program hello` prints `inject`/`provide`, so `manifest()` runs a
hello process per enabled entry (`core/loader.rs:632`). Key resolution therefore
lives in the profile: `.zirkle/default/init.lua` hand-wires `harness` to
`{"environment", "pty", "memory"}`, and `inject = {"tool.*"}` expands only
against what the profile's other entries already declare (`core/loader.rs:365`).
Realm isolation exists but is a profile-entry list folded into the context at
spawn (`core/loader.rs:671`, `core/runtime.rs:545`), so it describes an instance
rather than a dependency.

## Spec

Add `provide` and `needs` to `cartridge.json`. `provide` is a list of keys.
`needs` is a map from key to `{shared|private}`, defaulting to `shared`, so the
common case stays one line and isolation is opt-in — the default must be shared,
or a recursive tree grows a second `router`, `pty` or `sessions.dir` per subtree.
Globs stay legal in `needs` for surfaces (`tool.*`).

Resolution is a pure function over the folder scan: build a key to cartridge
index from the manifests of the cartridges available to the composition, then
walk `needs` from the entries the profile names. Ambiguity is an error, never a
silent pick; a profile entry's explicit `inject` or a named provider wins over
the index. Keep the index a value the loader computes before any fiber runs, so
it is testable without a host.

Declarations become verified, not merely trusted: at apply, compare the manifest's
`provide` with what the component registers and fail the entry on a mismatch.
That is what makes it safe for [[a-key-starts-its-provider-on-demand]] to trust
the manifest and skip the hello spawn.

`isolate` on the profile entry stays as the instance-level override, and the
declaration supplies its default. Both resolve through the same realm table
(`core/runtime.rs:184`), so nothing new is needed in the runtime.
