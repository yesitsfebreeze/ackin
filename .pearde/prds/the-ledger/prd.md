---
state: done
origin: requested
priority: 80
complexity: 25
blast-radius:
needs:
  - the-manifest
actual: 0.07h
---



# The ledger

The registry the resolver reads: every cartridge installed on this machine, what
each one provides, what each one needs.

What exists today resolves within a single profile's hand-listed
`.zirkle/<profile>/init.lua`; `deps()` in `src/main.rs` and the loader's
inject-to-provider search walk only that list. A cartridge that is present but
unlisted does not exist.

Replace that: the host scans a cartridge root, reads each manifest, and derives
what is available from what is there. A profile becomes an override on top of the
ledger rather than the manifest of record.

Auto-registration is the requirement. Installing a cartridge is putting it where
the ledger looks. Uninstalling it is taking it away. Neither involves editing a
list, because a list that must be edited is a second place for the truth to live.

The namespacing decision in [[the-manifest]] is **settled**, and it fixes this
PRD's data structure before a spec is written: *hidden until passed on*. The
ledger is therefore a **namespace of subtrees, not a flat table.** An entry is
identified by its path from the root, never by its bare name; two cartridges may
provide the same key without colliding; a key resolves against the asking
cartridge's subtree and walks outward; and a parent that wants an inner key
visible outside re-exports it by name in its own manifest. Removing a parent
removes its whole subtree, which is what makes the auto-registration above
symmetric — putting a tree where the ledger looks installs everything in it,
taking it away uninstalls everything in it.

## Questions

### Q1: What a newly installed cartridge does

You are choosing what happens the moment a cartridge is dropped into the folder
the machine watches: it starts running on its own, or it becomes available and
waits to be asked for. Whichever you pick is what someone gets from a fresh
install, before they have configured anything?

1. **Available, not started** — a new cartridge is listed and can be asked for, and nothing of it runs until something needs it. (recommended)
2. **Starts as soon as it is installed** — dropping one in brings it up straight away, and stopping it means taking it away again.
3. **Waits for your yes the first time** — each new cartridge shows up as something pending your approval before it is ever allowed to run.

<!-- for the board: loader.rs Host::derived, the `disabled` default; flipping it costs one assertion in src/tests/bridge.rs (init.lua rewritten without a folder stops it) -->

### Q2: Two cartridges offering the same thing

Two cartridges sitting side by side can both offer the same capability, and
something asking for it by name then has two answers. Right now one of them wins
quietly and the other is never reached; you are choosing whether that silence is
acceptable, or whether it should stop and say so?

1. **Say it is ambiguous and stop** — the ask fails naming both offers, so you find out when you install rather than from odd behaviour later. (recommended)
2. **The nearest one wins, quietly** — the closest offer is taken and anything further out with the same name is ignored without a word.
3. **You name the winner once** — the clash is reported and you settle it yourself, and the machine honours that from then on.

<!-- for the board: ledger.rs Ledger::resolve, the same-scope candidate list; reproduced at step 5b of probe/the-ledger-is-the-tree.sh (`store.get <- far` with `other` also offering it) -->

## Answers

**Q1** *(answered 2026-09-12 18:34)* — Available, not started — a new cartridge is listed and can be asked for, and nothing of it runs until something needs it.

**Q2** *(answered 2026-09-12 18:34)* — Say it is ambiguous and stop — the ask fails naming both offers, so you find out when you install rather than from odd behaviour later.

## History

**failed, retried 2026-09-12 18:48**

**2026-09-12 18:46 — the lane holds work the spec did not claim**

`lane/the-ledger` has 2 path(s) standing outside the footprint:

- `src/lib.rs`
- `src/tests/mod.rs`

Nothing merged and nothing lost: the paths stand in the lane. Widen the footprint in the spec or drop the files, then `pearde retry the-ledger` puts a worker back on that lane.

**failed, retried 2026-09-12 18:52**

spec01: exit 0

running 7 tests
test tests::ledger::a_root_that_does_not_exist_is_an_empty_ledger ... ok
test tests::ledger::an_unreadable_document_is_an_entry_with_its_reason ... ok
test tests::ledger::an_entry_is_its_path_from_the_root ... ok
test tests::ledger::two_entries_of_one_scope_offering_one_key_is_a_clash ... ok
test tests::ledger::a_new_cartridge_is_available_and_not_started ... ok
test tests::ledger::a_walk_answers_from_the_asker_subtree_then_steps_outward ... ok
test tests::ledger::a_walk_from_a_nested_scope_steps_outward ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 68 filtered out; finished in 0.01s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

src/ledger.rs:177:	pub fn resolve(&self, from: &str, key: &str) -> Bound<'_> {
src/ledger.rs:206:					.map(move |key| (e, key, self.resolve(&e.path, key)))
src/lua.rs:217:		let declared = crate::loader::resolve(path)?;
src/main.rs:292:			match ledger.resolve(&e.path, key) {
src/loader.rs:508:pub(crate) fn resolve(path: &Path) -> mlua::Result<Declared> {
== 1. the tree, with no list edited anywhere ==
far  exports store.get
far/deep  provides store.get
other  provides store.get
outer
  store.get <- outer/inner
outer/inner  provides store.get

== 2. the nearer provider wins ==
ok: outer's need bound to its own child, not to the unrelated top-level cartridge

== 3. installing is putting a tree where the ledger looks ==
ok: fresh appears with no list edited
ok: so does everything inside it

== 4. uninstalling is taking it away, and it takes its subtree ==
ok: fresh and fresh/child both gone
ok: the ledger is back to exactly what it was

== 5. a plain folder does not extend a subtree ==
ok: vendor/hidden is in no subtree; a non-cartridge folder is not descended into

== 5b. two entries of the SAME scope offering one key: ambiguous and stops ==
ok: the ask names both offers instead of a silent pick
asker
  store.get <- ambiguous (far, other)
far  exports store.get
far/deep  provides store.get
other  provides store.get
outer
  store.get <- outer/inner
outer/inner  provides store.get
ok: the exit is non-zero, so the clash is found at install time

== 6. an unreadable document is an entry, not an absence ==
broken  error: /var/folders/_p/tzmzw3m10kg7sg9hc7_mkm7w0000gn/T/tmp.SeTb14JIxU/cartridges/broken/cartridge.json: key must be a string at line 1 column 3
far  exports store.get
far/deep  provides store.get
other  provides store.get
outer
  store.get <- outer/inner
outer/inner  provides store.get
ok: broken is listed with its reason, and the exit is non-zero

== 7. a need nothing offers: named, and not a failure ==
ok: the need is named with ?
far  exports store.get
far/deep  provides store.get
lonesome
  nobody.offers <- ?
other  provides store.get
outer
  store.get <- outer/inner
outer/inner  provides store.get
ok: the exit is zero — the launch cares, not the registry

== 8. an empty root: nothing installed is a state, not an error ==
ok: an empty root prints nothing and exits zero
ok: so does a root that does not exist at all

== 9. the clash still stops after the two states above ==
ok: the clash is still named
asker
  store.get <- ambiguous (far, other)
far  exports store.get
far/deep  provides store.get
other  provides store.get
outer
  store.get <- outer/inner
outer/inner  provides store.get
ok: the exit is still non-zero — neither new case weakened the clash

PROBE OK
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.04s
     Running unittests src/lib.rs (target/debug/deps/zirkle-9dda9247c73c08cf)
     Running unittests src/main.rs (target/debug/deps/zirkle-10a290a4bef73b51)
    Checking zirkle v0.1.0 (/Users/feb/dev/cartridge/.pearde/.lanes/the-ledger)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.23s

spec02: exit 1

running 15 tests
test tests::manifest::an_export_that_names_nothing_inside_is_refused ... ok
test tests::manifest::an_empty_document_requests_nothing ... ok
test tests::manifest::a_disabled_cartridge_still_declares_what_it_asked_for ... ok
test tests::manifest::a_missing_file_the_document_names_does_not_blank_its_declarations ... ok
test tests::manifest::a_parent_passes_an_inner_key_outward_by_naming_it ... ok
test tests::manifest::malformed_declarations_are_refused_without_evaluating_the_entry ... ok
test tests::manifest::a_blank_grant_path_is_refused_like_a_blank_key ... ok
test tests::manifest::a_grandchild_key_reaches_the_top_only_through_every_level ... ok
test tests::manifest::the_capability_request_is_on_the_same_document ... ok
test tests::manifest::an_unreadable_document_asks_for_nothing_knowable_not_for_nothing ... ok
test tests::manifest::a_cartridge_two_levels_down_is_not_offered ... ok
test tests::manifest::the_document_declares_what_it_provides_and_needs ... ok
test tests::manifest::two_subtrees_may_provide_the_same_key ... ok
test tests::manifest::the_document_overrides_what_the_entry_declares ... ok
test tests::manifest::probe_nesting_is_not_discovered ... ok

test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 60 filtered out; finished in 0.01s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 1 test
test tests::ledger::a_new_cartridge_is_available_and_not_started ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 74 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Compiling zirkle v0.1.0 (/Users/feb/dev/cartridge/.pearde/.lanes/the-ledger)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.37s
     Running unittests src/lib.rs (target/debug/deps/zirkle-9dda9247c73c08cf)
     Running unittests src/main.rs (target/debug/deps/zirkle-10a290a4bef73b51)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.03s
     Running unittests src/lib.rs (target/debug/deps/zirkle-9dda9247c73c08cf)
     Running unittests src/main.rs (target/debug/deps/zirkle-10a290a4bef73b51)

## Report

spec01: exit 0

running 7 tests
test tests::ledger::a_root_that_does_not_exist_is_an_empty_ledger ... ok
test tests::ledger::an_unreadable_document_is_an_entry_with_its_reason ... ok
test tests::ledger::a_new_cartridge_is_available_and_not_started ... ok
test tests::ledger::an_entry_is_its_path_from_the_root ... ok
test tests::ledger::a_walk_answers_from_the_asker_subtree_then_steps_outward ... ok
test tests::ledger::a_walk_from_a_nested_scope_steps_outward ... ok
test tests::ledger::two_entries_of_one_scope_offering_one_key_is_a_clash ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 68 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

src/ledger.rs:177:	pub fn resolve(&self, from: &str, key: &str) -> Bound<'_> {
src/ledger.rs:206:					.map(move |key| (e, key, self.resolve(&e.path, key)))
src/lua.rs:217:		let declared = crate::loader::resolve(path)?;
src/main.rs:292:			match ledger.resolve(&e.path, key) {
src/loader.rs:508:pub(crate) fn resolve(path: &Path) -> mlua::Result<Declared> {
== 1. the tree, with no list edited anywhere ==
far  exports store.get
far/deep  provides store.get
other  provides store.get
outer
  store.get <- outer/inner
outer/inner  provides store.get

== 2. the nearer provider wins ==
ok: outer's need bound to its own child, not to the unrelated top-level cartridge

== 3. installing is putting a tree where the ledger looks ==
ok: fresh appears with no list edited
ok: so does everything inside it

== 4. uninstalling is taking it away, and it takes its subtree ==
ok: fresh and fresh/child both gone
ok: the ledger is back to exactly what it was

== 5. a plain folder does not extend a subtree ==
ok: vendor/hidden is in no subtree; a non-cartridge folder is not descended into

== 5b. two entries of the SAME scope offering one key: ambiguous and stops ==
ok: the ask names both offers instead of a silent pick
asker
  store.get <- ambiguous (far, other)
far  exports store.get
far/deep  provides store.get
other  provides store.get
outer
  store.get <- outer/inner
outer/inner  provides store.get
ok: the exit is non-zero, so the clash is found at install time

== 6. an unreadable document is an entry, not an absence ==
broken  error: /var/folders/_p/tzmzw3m10kg7sg9hc7_mkm7w0000gn/T/tmp.eQMossE52j/cartridges/broken/cartridge.json: key must be a string at line 1 column 3
far  exports store.get
far/deep  provides store.get
other  provides store.get
outer
  store.get <- outer/inner
outer/inner  provides store.get
ok: broken is listed with its reason, and the exit is non-zero

== 7. a need nothing offers: named, and not a failure ==
ok: the need is named with ?
far  exports store.get
far/deep  provides store.get
lonesome
  nobody.offers <- ?
other  provides store.get
outer
  store.get <- outer/inner
outer/inner  provides store.get
ok: the exit is zero — the launch cares, not the registry

== 8. an empty root: nothing installed is a state, not an error ==
ok: an empty root prints nothing and exits zero
ok: so does a root that does not exist at all

== 9. the clash still stops after the two states above ==
ok: the clash is still named
asker
  store.get <- ambiguous (far, other)
far  exports store.get
far/deep  provides store.get
other  provides store.get
outer
  store.get <- outer/inner
outer/inner  provides store.get
ok: the exit is still non-zero — neither new case weakened the clash

PROBE OK
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.03s
     Running unittests src/lib.rs (target/debug/deps/zirkle-9dda9247c73c08cf)
     Running unittests src/main.rs (target/debug/deps/zirkle-10a290a4bef73b51)
    Checking zirkle v0.1.0 (/Users/feb/dev/cartridge/.pearde/.lanes/the-ledger)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.97s

spec02: exit 0

running 15 tests
test tests::manifest::an_empty_document_requests_nothing ... ok
test tests::manifest::a_blank_grant_path_is_refused_like_a_blank_key ... ok
test tests::manifest::a_cartridge_two_levels_down_is_not_offered ... ok
test tests::manifest::an_export_that_names_nothing_inside_is_refused ... ok
test tests::manifest::a_missing_file_the_document_names_does_not_blank_its_declarations ... ok
test tests::manifest::malformed_declarations_are_refused_without_evaluating_the_entry ... ok
test tests::manifest::a_disabled_cartridge_still_declares_what_it_asked_for ... ok
test tests::manifest::a_grandchild_key_reaches_the_top_only_through_every_level ... ok
test tests::manifest::the_capability_request_is_on_the_same_document ... ok
test tests::manifest::an_unreadable_document_asks_for_nothing_knowable_not_for_nothing ... ok
test tests::manifest::the_document_declares_what_it_provides_and_needs ... ok
test tests::manifest::a_parent_passes_an_inner_key_outward_by_naming_it ... ok
test tests::manifest::the_document_overrides_what_the_entry_declares ... ok
test tests::manifest::probe_nesting_is_not_discovered ... ok
test tests::manifest::two_subtrees_may_provide_the_same_key ... ok

test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 60 filtered out; finished in 0.01s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 1 test
test tests::ledger::a_new_cartridge_is_available_and_not_started ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 74 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Compiling zirkle v0.1.0 (/Users/feb/dev/cartridge/.pearde/.lanes/the-ledger)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.32s
     Running unittests src/lib.rs (target/debug/deps/zirkle-9dda9247c73c08cf)
     Running unittests src/main.rs (target/debug/deps/zirkle-10a290a4bef73b51)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.03s
     Running unittests src/lib.rs (target/debug/deps/zirkle-9dda9247c73c08cf)
     Running unittests src/main.rs (target/debug/deps/zirkle-10a290a4bef73b51)

spec03: exit 0
== 1. the tree, with no list edited anywhere ==
far  exports store.get
far/deep  provides store.get
other  provides store.get
outer
  store.get <- outer/inner
outer/inner  provides store.get

== 2. the nearer provider wins ==
ok: outer's need bound to its own child, not to the unrelated top-level cartridge

== 3. installing is putting a tree where the ledger looks ==
ok: fresh appears with no list edited
ok: so does everything inside it

== 4. uninstalling is taking it away, and it takes its subtree ==
ok: fresh and fresh/child both gone
ok: the ledger is back to exactly what it was

== 5. a plain folder does not extend a subtree ==
ok: vendor/hidden is in no subtree; a non-cartridge folder is not descended into

== 5b. two entries of the SAME scope offering one key: ambiguous and stops ==
ok: the ask names both offers instead of a silent pick
asker
  store.get <- ambiguous (far, other)
far  exports store.get
far/deep  provides store.get
other  provides store.get
outer
  store.get <- outer/inner
outer/inner  provides store.get
ok: the exit is non-zero, so the clash is found at install time

== 6. an unreadable document is an entry, not an absence ==
broken  error: /var/folders/_p/tzmzw3m10kg7sg9hc7_mkm7w0000gn/T/tmp.5ccyLqFTXP/cartridges/broken/cartridge.json: key must be a string at line 1 column 3
far  exports store.get
far/deep  provides store.get
other  provides store.get
outer
  store.get <- outer/inner
outer/inner  provides store.get
ok: broken is listed with its reason, and the exit is non-zero

== 7. a need nothing offers: named, and not a failure ==
ok: the need is named with ?
far  exports store.get
far/deep  provides store.get
lonesome
  nobody.offers <- ?
other  provides store.get
outer
  store.get <- outer/inner
outer/inner  provides store.get
ok: the exit is zero — the launch cares, not the registry

== 8. an empty root: nothing installed is a state, not an error ==
ok: an empty root prints nothing and exits zero
ok: so does a root that does not exist at all

== 9. the clash still stops after the two states above ==
ok: the clash is still named
asker
  store.get <- ambiguous (far, other)
far  exports store.get
far/deep  provides store.get
other  provides store.get
outer
  store.get <- outer/inner
outer/inner  provides store.get
ok: the exit is still non-zero — neither new case weakened the clash

PROBE OK

running 75 tests
test tests::bridge::process_scripts_resolve_relative_to_the_cartridge ... ok
test tests::bridge::bridge_manifest_rejects_paths_outside_the_cartridge ... ok
test tests::cartridges::a_changed_file_swaps_the_fiber_and_a_broken_one_keeps_it ... ok
test tests::cartridges::config_lua_overrides_entries_and_manifest_resolves_providers ... ok
test tests::cartridges::a_glob_injects_every_matching_key_the_other_entries_provide ... ok
test tests::bridge::bridge_status_tracks_active_generations_and_scopes_backend_calls ... ok
test tests::contracts::a_broken_integration_check_fails_closed_with_its_error ... ok
test tests::contracts::a_cartridge_declaring_nothing_is_never_called ... ok
test tests::contracts::a_broken_selftest_fails_closed_and_names_the_cartridge ... ok
test tests::contracts::declared_contracts_run_after_apply_and_pass ... ok
test tests::contracts::one_obligation_may_be_declared_alone ... ok
test tests::folders::malformed_manifests_fail_without_evaluating_entries ... ok
test tests::ledger::a_new_cartridge_is_available_and_not_started ... ok
test tests::ledger::a_root_that_does_not_exist_is_an_empty_ledger ... ok
test tests::folders::folder_manifest_names_the_component_and_reloads_its_entry ... ok
test tests::ledger::a_walk_answers_from_the_asker_subtree_then_steps_outward ... ok
test tests::ledger::a_walk_from_a_nested_scope_steps_outward ... ok
test tests::ledger::an_entry_is_its_path_from_the_root ... ok
test tests::ledger::an_unreadable_document_is_an_entry_with_its_reason ... ok
test tests::ledger::two_entries_of_one_scope_offering_one_key_is_a_clash ... ok
test tests::lifecycle::a_raising_step_fails_the_fiber_with_nothing_installed ... ok
test tests::lifecycle::a_cycle_stays_inactive ... ok
test tests::foreground::foreground_call_returns_the_service_value_and_disposes_the_profile ... ok
test tests::cartridges::yields_are_boundaries_and_access_is_enforced ... ok
test tests::cartridges::reconcile_adds_removes_and_revises_by_id ... ok
test tests::foreground::foreground_missing_or_failed_service_still_disposes ... ok
test tests::lifecycle::effects_revert_in_lifo_order ... ok
test tests::lifecycle::disposing_a_parent_retires_its_children ... ok
test tests::lifecycle::listeners_fire_in_order_and_bail_stops ... ok
test tests::lifecycle::user_values_and_callbacks_are_dropped_outside_the_registry_lock ... ok
test tests::manifest::a_blank_grant_path_is_refused_like_a_blank_key ... ok
test tests::manifest::a_cartridge_two_levels_down_is_not_offered ... ok
test tests::manifest::a_disabled_cartridge_still_declares_what_it_asked_for ... ok
test tests::manifest::a_grandchild_key_reaches_the_top_only_through_every_level ... ok
test tests::manifest::a_missing_file_the_document_names_does_not_blank_its_declarations ... ok
test tests::manifest::a_parent_passes_an_inner_key_outward_by_naming_it ... ok
test tests::manifest::an_empty_document_requests_nothing ... ok
test tests::manifest::an_export_that_names_nothing_inside_is_refused ... ok
test tests::manifest::an_unreadable_document_asks_for_nothing_knowable_not_for_nothing ... ok
test tests::lifecycle::a_target_change_stops_the_iterator_at_the_boundary ... ok
test tests::manifest::malformed_declarations_are_refused_without_evaluating_the_entry ... ok
test tests::manifest::probe_nesting_is_not_discovered ... ok
test tests::manifest::the_capability_request_is_on_the_same_document ... ok
test tests::manifest::the_document_declares_what_it_provides_and_needs ... ok
test tests::manifest::the_document_overrides_what_the_entry_declares ... ok
test tests::manifest::two_subtrees_may_provide_the_same_key ... ok
test tests::process::closed_links_release_pending_and_reject_new_requests ... ok
test tests::process::disabled_processes_never_run_hello_apply_or_replace ... ok
test tests::process::dropping_a_waiter_ignores_its_late_reply ... ok
test tests::process::invalid_registration_fails_before_changing_loaded_fibers ... ok
test tests::lifecycle::access_is_enforced_at_the_point_of_use ... ok
test tests::lifecycle::isolated_realms_bind_independently ... ok
test tests::lifecycle::dependents_are_drained_before_any_provider_inverse ... ok
test tests::lifecycle::a_replaced_provider_reloads_its_dependents ... ok
test tests::cartridges::yolo_overrides_cartridge_config_only_for_automatic_services ... ok
test tests::reload::corrected_code_can_recover_a_failed_initial_generation ... ok
test tests::reload::switching_keeps_consumers_bound_and_rejects_bad_migrations ... ok
test tests::reload::two_entries_of_one_file_each_get_their_own_switch ... ok
test tests::process::sdk_eof_and_dispose_release_waiters_before_finalizers ... ok
test tests::rpc_contract::rust_sdk_and_lua_share_json_and_bidirectional_contracts ... ok
test tests::process::sdk_child_roundtrip_errors_metadata_and_eof ... ok
test tests::rpc_contract::rust_sdk_eof_rejects_another_inflight_call ... ok
test tests::socket::one_turn_id_crosses_the_socket_the_host_a_cartridge_and_lua ... ok
test tests::socket::socket_calls_a_lua_wrapped_sdk_process_with_correlated_errors ... ok
test turn::tests::a_credential_or_a_prompt_body_is_omitted_and_named ... ok
test tests::socket::a_client_round_trips_through_a_cartridge ... ok
test turn::tests::the_sink_stays_bounded_by_rotating_one_generation ... ok
test tests::rpc_contract::rust_sdk_failed_apply_retains_generation_then_reload_and_dispose_work ... ok
test tests::process::ordinary_lua_composition_can_load_a_process_wrapper ... ok
test tests::process::lua_wrappers_merge_injections_before_start_and_preserve_config ... ok
test tests::folders::watcher_reloads_a_folder_when_its_manifest_changes ... ok
test tests::cartridges::wrapped_process_isolation_metadata_and_dependency_restart_compose ... ok
test tests::process::a_process_provides_listens_and_is_disposed ... ok
test tests::cartridges::local_list_resolves_wrappers_without_starting_a_daemon_or_applying_cartridges ... ok
test tests::process::watches_reload_wrappers_binaries_and_new_executable_directories_once ... ok

test result: ok. 75 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.31s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Compiling zirkle v0.1.0 (/Users/feb/dev/cartridge/.pearde/.lanes/the-ledger)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.14s
     Running unittests src/lib.rs (target/debug/deps/zirkle-9dda9247c73c08cf)
{"msg":"syntax error: [string \"/private/var/folders/_p/tzmzw3m10kg7sg9hc7_mk...\"]:1: syntax error near 'is'","src":"p","t":1789232161537,"turn":null}
{"msg":"syntax error: [string \"/private/var/folders/_p/tzmzw3m10kg7sg9hc7_mk...\"]:1: syntax error near 'Lua'","src":"p","t":1789232161538,"turn":null}
{"msg":"runtime error: /private/var/folders/_p/tzmzw3m10kg7sg9hc7_mkm7w0000gn/T/.tmpVyKlm4/p/cartridge.json: expected value at line 1 column 1","src":"instance","t":1789232161546,"turn":null}
{"msg":"runtime error: [string \"/private/var/folders/_p/tzmzw3m10kg7sg9hc7_mk...\"]:3: migration rejected\nstack traceback:\n\t[C]: in function 'error'\n\t[string \"/private/var/folders/_p/tzmzw3m10kg7sg9hc7_mk...\"]:3: in function <[string \"/private/var/folders/_p/tzmzw3m10kg7sg9hc7_mk...\"]:1>","src":"p","t":1789232161690,"turn":null}
{"msg":"rejected fixture migration","src":"peer","t":1789232161988,"turn":null}
     Running unittests src/main.rs (target/debug/deps/zirkle-10a290a4bef73b51)
   Doc-tests zirkle
