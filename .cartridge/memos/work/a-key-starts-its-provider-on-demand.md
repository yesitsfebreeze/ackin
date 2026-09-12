---
kind: work
description: "A composed profile starts only the chain something reaches, and declarations are read without spawning anything"
status: open
level: 10
estimate: 2d
needs:
  - "[[a-cartridge-declares-what-it-needs]]"
  - "[[the-profile-is-the-root-composer]]"
---

# a-key-starts-its-provider-on-demand

## Outcome

Composing a profile does not start it. Declarations are read from manifests, and
a cartridge is started when something first reaches a key it provides — the
chain that key needs starting with it, and nothing else. A key that no cartridge
provides still fails immediately with the same named error as today, rather than
waiting for a timeout. A cartridge the profile enables but nothing reaches costs
one manifest read.

Scope is first-touch activation. Stopping an idle subtree again is deliberately
out of scope; nothing in this outcome depends on it.

## Check

- [ ] `zirkle run tool.memo '{"op":"types"}'` against the default profile starts
      `memo` and its chain and starts no `router`, `gitfs`, `pty` or `memory`
      process, asserted from the spawned-executable list, and prints the same
      reply as before.
- [ ] The same run spawns no `hello` process for an unreached cartridge.
- [ ] `zirkle run` for a key no cartridge provides fails with a message naming
      the key, without waiting out a timeout.
- [ ] A key whose provider needs a second cartridge activates both, in
      dependency order, proven by the fibers' transition events.
- [ ] `zirkle run ui` still brings up the full surface: reaching `ui` reaches
      everything it needs, so the terminal starts with the same services active
      as today, asserted from the landscape rows.
- [ ] Concurrent first touches of one key activate its provider once, proven by
      a single apply under a parallel-call test.
- [ ] `just check` and `just test` pass.

## Approach

Observed 2026-09-12. `reconcile` instantiates every enabled entry and
`instantiate` spawns each one immediately (`core/loader.rs:703`, `:677`); reading
declarations runs `program hello` per enabled process cartridge
(`core/loader.rs:632`). `run_mode` then waits until some fiber both is active and
provides the key, and otherwise reports the stalled set (`core/loader.rs:553`).
So a `tool.memo` call today pays for the whole default profile, and an absent key
is distinguishable from a slow one only by the 30-second timeout. The runtime is
already reactive on key availability, and `Ctx::get` returns `Error::Inactive`
for a key with no active provider (`core/runtime.rs:527`) — that error is the
exact place where activation belongs instead.

## Spec

Two changes, in this order.

First, declarations without execution. With `provide` and `needs` in the manifest
from [[a-cartridge-declares-what-it-needs]], the composer builds its graph from
the folder scan alone. `hello` stops being a boot step and becomes what verifies
a cartridge at apply.

Second, demand activation. The composer registers each resolved key as claimable
by its provider without instantiating it. A `get` or service call for an
unactivated key activates that provider's subtree and waits for it, so the
first touch pays the start cost and later ones do not; a key with no registered
claim fails at once rather than timing out. Activation is single-flight per
entry: concurrent touches join one apply. `run_mode`'s wait loop reduces to
activating the requested key and awaiting that one fiber, and `stalled` keeps
reporting only entries that were asked for and failed, so a diagnostic never
starts anything.

Entries that must run without being called — the wrapped shell, the socket, the
UI — stay eager, declared as such by the profile entry rather than inferred.
`zirkle list` and the landscape report declared-but-unstarted as a state, so the
distinction between "not composed", "not started" and "failed" stays visible.
