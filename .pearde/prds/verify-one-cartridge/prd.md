---
state: done
origin: requested
priority: 55
complexity: 8
blast-radius: high
needs:
  - the-wire
workflow: drive-the-binary
actual: 0.01h
commit: ff37e80 e662bf0
---


# Verify one cartridge

Each piece verified on its own, against its own contract, before it is allowed
to matter.

`zirkle verify` already loads a profile and runs every contract its cartridges
declare — the `Command::Verify` arm in `src/main.rs`. What this PRD makes true is
that verification is meaningful for a single cartridge in isolation: you can test
the thing you just wrote without assembling a system around it, because
[[the-ledger]] can resolve its dependencies and [[the-manifest]] states its
contract.

This is what closes the loop the whole design exists for. Write a cartridge,
verify it alone, slot it in. The tool belt gets better because each addition is
cheap to trust, not because anyone audited the whole.

At the end, one cartridge is verifiable without the rest of the tree, and a
failing contract keeps it out.

## History

**failed, retried 2026-09-12 20:40**

spec01: exit 1

running 12 tests
test tests::contracts::an_unknown_cartridge_is_named_rather_than_run ... ok
test tests::contracts::a_clashed_need_stops_the_run_before_it_loads ... ok
test tests::contracts::a_need_that_binds_to_nothing_is_named_instead_of_run ... ok
test tests::contracts::a_failing_contract_names_the_path_the_obligation_and_the_key ... ok
test tests::contracts::a_broken_selftest_fails_closed_and_names_the_cartridge ... ok
test tests::contracts::one_cartridge_is_verified_without_a_profile ... ok
test tests::contracts::declared_contracts_run_after_apply_and_pass ... ok
test tests::contracts::a_cartridge_declaring_nothing_is_never_called ... ok
test tests::contracts::a_nested_cartridge_is_verified_by_its_path_from_the_root ... ok
test tests::contracts::a_broken_integration_check_fails_closed_with_its_error ... ok
test tests::contracts::the_providers_a_need_binds_to_are_reached_but_never_graded ... ok
test tests::contracts::one_obligation_may_be_declared_alone ... ok

test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 107 filtered out; finished in 0.02s

   Compiling libc v0.2.189
   Compiling proc-macro2 v1.0.107
   Compiling unicode-ident v1.0.24
   Compiling quote v1.0.47
   Compiling cfg-if v1.0.4
   Compiling serde_core v1.0.229
   Compiling find-msvc-tools v0.1.12
   Compiling shlex v2.0.1
   Compiling autocfg v1.5.1
   Compiling memchr v2.8.3
   Compiling bitflags v2.13.2
   Compiling parking_lot_core v0.9.12
   Compiling num-traits v0.2.19
   Compiling cc v1.4.5
   Compiling scopeguard v1.2.0
   Compiling utf8parse v0.2.2
   Compiling serde v1.0.229
   Compiling typeid v1.0.3
   Compiling lua-src v551.0.2
   Compiling pin-project-lite v0.2.17
   Compiling smallvec v1.16.1
   Compiling futures-core v0.3.34
   Compiling pkg-config v0.3.34
   Compiling futures-sink v0.3.34
   Compiling futures-channel v0.3.34
   Compiling anstyle-parse v1.0.0
   Compiling lock_api v0.4.14
   Compiling zmij v1.0.23
   Compiling futures-io v0.3.34
   Compiling is_terminal_polyfill v1.70.2
   Compiling erased-serde v0.4.10
   Compiling slab v0.4.12
   Compiling rustix v1.1.4
   Compiling getrandom v0.4.3
   Compiling anstyle v1.0.14
   Compiling colorchoice v1.0.5
   Compiling anstyle-query v1.1.5
   Compiling futures-task v0.3.34
   Compiling anstream v1.0.0
   Compiling which v8.0.6
   Compiling luajit-src v210.7.3+1ee778a
   Compiling errno v0.3.14
   Compiling mlua-sys v0.12.0
   Compiling syn v3.0.5
   Compiling futures-macro v0.3.34
   Compiling serde_derive v1.0.229
   Compiling futures-util v0.3.34
   Compiling parking_lot v0.12.5
   Compiling objc2-core-foundation v0.3.2
   Compiling heck v0.5.0
   Compiling serde_json v1.0.151
   Compiling same-file v1.0.6
   Compiling notify v9.0.0-rc.5
   Compiling strsim v0.11.1
   Compiling clap_lex v1.1.0
   Compiling thiserror v2.0.20
   Compiling ordered-float v2.10.1
   Compiling clap_builder v4.6.6
   Compiling walkdir v2.5.0
   Compiling clap_derive v4.6.4
   Compiling objc2-core-services v0.3.2
   Compiling bstr v1.13.1
   Compiling tokio-macros v2.7.2
   Compiling thiserror-impl v2.0.20
   Compiling signal-hook-registry v1.4.8
   Compiling socket2 v0.6.5
   Compiling mio v1.2.3
   Compiling notify-types v2.1.0
   Compiling itoa v1.0.18
   Compiling rustc-hash v2.1.3
   Compiling bytes v1.12.1
   Compiling xxhash-rust v0.8.18
   Compiling log v0.4.34
   Compiling once_cell v1.21.4
   Compiling either v1.18.0
   Compiling fastrand v2.5.0
   Compiling tempfile v3.27.0
   Compiling tokio v1.53.1
   Compiling clap v4.6.6
   Compiling futures-executor v0.3.34
   Compiling futures v0.3.34
   Compiling serde-value v0.7.0
   Compiling mlua v0.12.1
   Compiling zirkle v0.1.0 (/Users/feb/dev/cartridge/.pearde/.sessions/s25699)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 9.78s
     Running unittests src/lib.rs (target/debug/deps/zirkle-9dda9247c73c08cf)
   Compiling zirkle v0.1.0 (/Users/feb/dev/cartridge/.pearde/.sessions/s25699)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3.42s
bash: line 2: .pearde/prds/verify-one-cartridge/probe/verify-one-cartridge.sh: No such file or directory

## Report

spec01: exit 0

running 12 tests
test tests::contracts::an_unknown_cartridge_is_named_rather_than_run ... ok
test tests::contracts::a_need_that_binds_to_nothing_is_named_instead_of_run ... ok
test tests::contracts::a_clashed_need_stops_the_run_before_it_loads ... ok
test tests::contracts::declared_contracts_run_after_apply_and_pass ... ok
test tests::contracts::a_failing_contract_names_the_path_the_obligation_and_the_key ... ok
test tests::contracts::a_broken_selftest_fails_closed_and_names_the_cartridge ... ok
test tests::contracts::one_cartridge_is_verified_without_a_profile ... ok
test tests::contracts::a_cartridge_declaring_nothing_is_never_called ... ok
test tests::contracts::a_broken_integration_check_fails_closed_with_its_error ... ok
test tests::contracts::a_nested_cartridge_is_verified_by_its_path_from_the_root ... ok
test tests::contracts::the_providers_a_need_binds_to_are_reached_but_never_graded ... ok
test tests::contracts::one_obligation_may_be_declared_alone ... ok

test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 107 filtered out; finished in 0.02s

== 1. verify the one cartridge, no profile anywhere
2 contracts passed
exit: 0
== 2. the failing neighbour is kept out of the one-cartridge run
exit: 0
== 3. the same tree with a broken contract: the run names it and fails
exit: 1
== 4. a need nothing offers is named before anything loads
exit: 1
== 5. a name that is not a cartridge is an error, not a run
exit: 1
== 6. every contract of the whole profile still behaves as before
0 contracts passed
exit: 0
== 7. the ledger still lists the tree
broken
log  provides log.write
store  provides store.get, store.check, store.wire
  log.write <- log
exit: 0
   Compiling zirkle v0.1.0 (/Users/feb/dev/cartridge/.pearde/.sessions/s25699)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.43s
     Running unittests src/lib.rs (target/debug/deps/zirkle-9dda9247c73c08cf)
   Compiling zirkle v0.1.0 (/Users/feb/dev/cartridge/.pearde/.sessions/s25699)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.44s
store selftest `store.check` returned false
store: need `absent.key` binds to nothing in the tree
`absent` is not a cartridge under /private/var/folders/_p/tzmzw3m10kg7sg9hc7_mkm7w0000gn/T/tmp.JCTB6mGSJ7/tree
