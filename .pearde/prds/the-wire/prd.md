---
state: done
origin: requested
priority: 75
complexity: 22
blast-radius: mid
needs:
  - the-resolver
workflow: nest-the-wire
actual: 0h
---


# The wire

How a node talks to the dependency that launched it.

The channel is the connection it was born with: inherited pipes carrying JSON
lines on stdin and stdout, the protocol already specified in the module header
of `src/cartridge.rs` and implemented on the cartridge side in `src/sdk.rs`. No
discovery, no broker, no address, no registry lookup at call time. A node that
needs its dependency is already holding it.

The existing protocol is very nearly symmetric — both halves can `call` and
both can `reply`. That near-symmetry is what makes recursion possible without
inventing a second protocol: a cartridge that speaks the host half of this wire
to children of its own **is** a sub-host, and a tree can nest without the binary
above it knowing that it nested. Closing the remaining asymmetry is this PRD's
work.

In scope: whether the Unix socket in `src/socket.rs` stays the only transport.
Socket-only is correct for POSIX and rules out both cross-machine cartridges and
Windows. A decision either way belongs in a memo.

At the end, a cartridge can host cartridges over the same wire it is itself
hosted on.

## History

**failed, retried 2026-09-12 20:18**

**2026-09-12 20:16 — the lane holds work the spec did not claim**

`lane/the-wire` has 6 path(s) standing outside the footprint:

- `Cargo.toml`
- `src/cartridge.rs`
- `src/sdk.rs`
- `src/tests/fixtures/nested_fixture.rs`
- `src/tests/mod.rs`
- `src/tests/wire.rs`

Nothing merged and nothing lost: the paths stand in the lane. Widen the footprint in the spec or drop the files, then `pearde retry the-wire` puts a worker back on that lane.

## Report

spec01: exit 0

running 6 tests
test tests::wire::a_daemon_reload_carries_down_to_the_child_that_declared_it ... ok
test tests::wire::a_cartridge_hosts_a_cartridge_over_the_same_wire ... ok
test tests::wire::a_child_key_the_sub_host_never_declared_stays_private ... ok
test tests::wire::a_child_that_never_becomes_ready_fails_the_spawn_where_it_failed ... ok
test tests::wire::a_nested_bridge_call_resolves_against_the_daemons_profile_grant ... ok
test tests::wire::a_child_that_declared_no_reload_still_answers_null ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 89 filtered out; finished in 1.79s

   Compiling zirkle v0.1.0 (/Users/feb/dev/cartridge/.pearde/.lanes/the-wire)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.36s
     Running unittests src/lib.rs (target/debug/deps/zirkle-9dda9247c73c08cf)
{"msg":"child refused reload","src":"parent","t":1789237089041,"turn":null}

spec02: exit 0

running 6 tests
test tests::wire::a_child_key_the_sub_host_never_declared_stays_private ... ok
test tests::wire::a_daemon_reload_carries_down_to_the_child_that_declared_it ... ok
test tests::wire::a_cartridge_hosts_a_cartridge_over_the_same_wire ... ok
test tests::wire::a_child_that_declared_no_reload_still_answers_null ... ok
test tests::wire::a_child_that_never_becomes_ready_fails_the_spawn_where_it_failed ... ok
test tests::wire::a_nested_bridge_call_resolves_against_the_daemons_profile_grant ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 89 filtered out; finished in 1.32s

    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.02s
     Running unittests src/lib.rs (target/debug/deps/zirkle-9dda9247c73c08cf)
{"msg":"child refused reload","src":"parent","t":1789237091001,"turn":null}
