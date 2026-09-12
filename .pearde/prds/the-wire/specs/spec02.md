---
complexity: 8
footprint:
  - src/sdk.rs
  - src/tests/wire.rs
---

# spec02 — a sub-host fronts what it declares, and carries reload down to its children

Three leftovers the probe's permissive forwarding left open, all inside
`sdk.rs`:

1. **Declared fronting.** `Host` does not retain the declaration its
   `Cartridge` carries, so `relay` re-advertises every key a child provides,
   declared or not — an undeclared child key leaks into the daemon's store.
   `Cartridge::run` must hand the declaration into `Host`, and `relay`
   re-advertises only provided keys the declaration names; an undeclared child
   key stays a private service of the sub-host.
2. **Reload reaches the nest.** The daemon's reload machinery asks the
   sub-host's link with `{"reload": …}`; today the parent answers from its own
   handler table or null and the child's `reload: true` manifest is never
   consulted. A child whose `hello` declared reload should receive the
   parent's reload frame down its own link, with the parent answering above
   from the child's reply.
3. **Bridge calls are forwarded, not refused.** A child's
   `{"bridge":"call", …}` frame currently falls through to the unknown-message
   error. The parent holds no bridge of its own, so forward it up: an
   `sdk::Host::bridge_call` pass-through that requests the same frame on the
   parent's link, with the profile check happening where it already lives, in
   the daemon.

## Acceptance

- [x] a child that provides a key its sub-host never declared stays private:
      the daemon's store never gains the key, and a call of it errs with
      "`<key>` is not provided" above the sub-host — closed by
      `cargo test --lib tests::wire::`,
      `tests::wire::a_child_key_the_sub_host_never_declared_stays_private ...
      ok` (asserts the call errs exactly `` `secret` is not provided ``)
- [x] a child whose `hello` declared `reload: true` answers a daemon reload
      request with the child's own reply, and a child that declared nothing
      still answers `null` — closed by `cargo test --lib tests::wire::`:
      `tests::wire::a_daemon_reload_carries_down_to_the_child_that_declared_it
      ... ok` (the child's `child refused reload` arrives as the daemon's
      report and the live generation stays; the ok answer switches it) and
      `tests::wire::a_child_that_declared_no_reload_still_answers_null ... ok`
      (the reload finishes with no error frame)
- [x] a nested cartridge's bridge call resolves against the daemon's profile
      grant exactly as a first-generation cartridge's does, error text
      included — closed by `cargo test --lib tests::wire::`,
      `tests::wire::a_nested_bridge_call_resolves_against_the_daemons_profile_grant
      ... ok` (with the grant the forwarded frame answers `3` through `p`'s
      `counter`; without it the daemon's own text
      `profile has not granted bridge access` comes back)

## Verify and Proof

```sh
cd /Users/feb/dev/cartridge/.pearde/.lanes/the-wire
cargo test --lib tests::wire::
```