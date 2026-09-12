---
complexity: 8
footprint:
  - src/main.rs
  - src/lua.rs
  - src/loader.rs
  - src/tests/contracts.rs
---

# spec01 — `zirkle verify <cartridge>`: one cartridge, in isolation, against its own contract

Naming a cartridge on `verify` runs just that cartridge's contracts: the ledger
scans the cartridge root, the target is found by its path from the root (or by
the folder it sits in), and the host loads exactly the target plus the
cartridges its needs bind to, resolved by the ledger's outward walk. The
profile is never read, so nothing of a system has to be assembled around the
cartridge just written. A need nothing offers, or a need one scope offers
twice, is named and the run never loads. The contracts come from the document
(`selftest`, `integration`), are the target's alone — providers pulled in are
reached, never graded — and a failing one is a line naming cartridge,
obligation and key, with exit 1: the failing contract keeps the cartridge out
of the slot-in loop. The armless form, `zirkle verify`, is untouched and keeps
answering for the whole profile.

**What already stands** (built in the lane, uncommitted): `Host::solo` holds the
caller's entry set in place of the profile's; `Host::verify_one` / `solo` /
`solo_entry` / `run_contracts` in `src/loader.rs` (the old `verify` is
`run_contracts(contracts())`); the `Command::Verify { cartridge }` arm in
`src/main.rs`; twelve contract tests in `src/tests/contracts.rs`; the probe
`prds/verify-one-cartridge/probe/verify-one-cartridge.sh` runs the loop end to
end, seven steps, all green. **Left to finish**: nothing inside this unit —
the implementer's job is landing what stands and closing the boxes below.

## Acceptance

- [x] `zirkle --dir <root> verify store` on a tree with no `init.lua` anywhere
      prints `2 contracts passed` and exits 0, where the two contracts are the
      document's `selftest` and `integration`, the integration reaching the
      real provider the ledger bound (`store: need log.write -> log`)
      — closed by `sh .pearde/prds/verify-one-cartridge/probe/verify-one-cartridge.sh`
      step 1: `2 contracts passed` / `exit: 0` on a `mktemp -d` tree holding
      only `cartridge.json` documents and Lua, no profile anywhere.
- [x] the same ask exits 1 printing `store selftest \`store.check\` returned
      false` after the contract is turned to `return false end`, and the line
      names the ledger path, the obligation and the key
      — closed by the same probe, step 3 (contract turned to
      `return false end`): `store selftest \`store.check\` returned false` /
      `exit: 1` — cartridge path, obligation, key, in that order.
- [x] with `needs: ["absent.key"]` in the document, `verify store` exits 1
      printing `store: need \`absent.key\` binds to nothing in the tree` and
      never loads the cartridge
      — closed by the same probe, step 4 (needs rewritten to `["absent.key"]`
      and back): the line printed, `exit: 1`, nothing else ran.
- [x] with two cartridges of one scope declaring the same `provide`, `verify
      store` exits 1 printing `store: need \`provider\` is ambiguous
      (provider, twin)` and never loads either
      — closed by
      `sh .pearde/prds/verify-one-cartridge/probe/ambiguous-need.sh` (fixture
      added beside the pass-one probe this pass): the line printed, `exit: 1`,
      neither side loaded. The unit test behind it,
      `a_clashed_need_stops_the_run_before_it_loads`, is green.
- [x] `verify absent` exits 1 printing "`absent` is not a cartridge under
      <root>" — an ask that names nothing is an error, not a run that graded
      nothing
      — closed by the same probe, step 5: `` `absent` is not a cartridge under
      <mktemp root> `` / `exit: 1`.
- [x] a neighbour of the target that declares a failing `selftest` is not
      graded: `verify store` passes on a tree whose `broken` cartridge's
      contract would fail
      — closed by the same probe, step 2: the tree's `broken` cartridge
      returns false, and `verify store` still exits 0 (step 1's pass).
- [x] `verify store` on a tree with a failing-contract neighbour and a broken
      `outer` (a folder with no document) still finds `outer/inner` by its
      path from the root; `cargo test` runs 82 tests, all green, the twelve in
      `tests::contracts` among them, including
      — closed by `cargo test --lib tests::contracts` in the lane: 12 passed,
      0 failed, and `cargo test` over the whole lane: 119 passed, 0 failed.
      The count moved from the spec's 82 to 119 because the lane was rebased
      onto `main` after the specs were written (`hot-reload` and `the-wire`
      work landed first); the twelve named tests below are all present and
      green. Nested-by-path is the unit test's own claim
      (`a_nested_cartridge_is_verified_by_its_path_from_the_root`), the probe
      walks `store/inner` the same way. Tests including:
      `one_cartridge_is_verified_without_a_profile`,
      `the_providers_a_need_binds_to_are_reached_but_never_graded`,
      `a_need_that_binds_to_nothing_is_named_instead_of_run`,
      `a_clashed_need_stops_the_run_before_it_loads`,
      `a_nested_cartridge_is_verified_by_its_path_from_the_root`,
      `a_failing_contract_names_the_path_the_obligation_and_the_key`,
      `an_unknown_cartridge_is_named_rather_than_run`
- [x] the armless `zirkle verify` on the same tree still answers for the whole
      profile as before (`0 contracts passed` with no profile, exit 0)
      — closed by the same probe, step 6: `0 contracts passed` / `exit: 0`;
      `ledger` (step 7) still lists the tree.

## Verify and Proof

```sh
cargo test --lib tests::contracts
cargo build
# absolute: collect runs this in the session checkout, where the board's
# .pearde/ does not exist; the script pins its own binary by its own path
/Users/feb/dev/cartridge/.pearde/prds/verify-one-cartridge/probe/verify-one-cartridge.sh
```