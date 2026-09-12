---
complexity: 14
footprint:
  - src/sdk.rs
  - src/cartridge.rs
  - src/tests/wire.rs
  - src/tests/mod.rs
  - src/tests/fixtures/nested_fixture.rs
  - Cargo.toml
---

# spec01 — a cartridge speaks the host half of the wire to cartridges of its own

`sdk::Host::spawn(name, cmd, config)` probes the child's `hello` (the
`cartridge::manifest` probe, now shared by both halves), spawns it with piped
stdio, sends `{"apply": …}` and holds until `ready`. A per-child reader task —
`relay` — is the host half of the wire, the inverse of the run loop: the
child's `provide` becomes a forwarding service plus a re-advertised
`{"provide": key}` up to the daemon, `on` becomes a parent listener that
requests the event down, `emit`/`send` forward up, `call`/`meta`/`injections`/
`cartridges`/`bridge status` answer by calling up, `reload` answers from the
parent's own reload handlers, and every upward request carries the child
frame's turn. A bare `error` frame from the child is re-reported upward; EOF
while the child is still wanted is reported as `<name>: exited`. Disposal is a
finalizer: `{"dispose": true}` down, the child waited out (5s, then killed), so
a disposed sub-host takes its tree with it.

This unit stands in the lane (built, not committed — `prds/the-wire/probe/`
holds the copy); the boxes are what an implementer re-verifies and a later
worker must not break.

## Acceptance

- [x] a sub-hosted child's provided key answers a daemon call through three
      wire hops (daemon → parent → child → daemon) with the child's answer
      assembled from keys the daemon injected into the parent — closed by
      `cargo test --lib tests::wire::`,
      `tests::wire::a_cartridge_hosts_a_cartridge_over_the_same_wire ... ok`
- [x] a child's `on` listener fires from a host emit and the child's `send`
      leaves through the parent to the socket audience — same test,
      `next_event(rx, "observed") == json!(1)` asserted inside it
- [x] disposing the sub-host runs the child's own finalizers and its farewell
      still reaches the socket through the parent, and a later call on the
      fronted key errs — same test: `next_event(rx, "disposed") == json!(true)`
      then the call errs; closed by `cargo test --lib tests::wire::`
- [x] a child that exits before `ready` fails the spawn with its exit status
      named, and an unreadable line shuts the child link down — closed by
      `cargo test --lib tests::wire::`,
      `tests::wire::a_child_that_never_becomes_ready_fails_the_spawn_where_it_failed
      ... ok` (asserts `nested exited before ready: exit status: 7` and
      `nested sent an unreadable line: garbage`)

## Verify and Proof

```sh
cd /Users/feb/dev/cartridge/.pearde/.lanes/the-wire
cargo test --lib tests::wire::
```

The run that wrote this spec: 76 passed, 0 failed, `tests::wire::
a_cartridge_hosts_a_cartridge_over_the_same_wire ... ok` — full suite at
`cargo test`.
## Proof (mason, 2026-09-12)

```sh
cd /Users/feb/dev/cartridge/.pearde/.lanes/the-wire
cargo test --lib tests::wire::
```
→ `test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 75 filtered out;
finished in 1.39s`. Full suite `cargo test` → `81 passed; 0 failed`. A fresh
recompile confirmed (`Compiling zirkle` after touching `src/lib.rs`), no cache
hit. One standing defect this pass fixed inside the footprint: the dispose
finalizer never set the reader's `stopping` flag, so a disposed child's death
was reported upward as `<name>: exited` though the code comment claimed
otherwise — the finalizer now stores the flag before sending `dispose`.
