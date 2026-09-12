---
state: done
origin: requested
priority: 95
complexity: 25
blast-radius: mid
needs:
  - the-ledger
actual: 0.06h
commit: 62d679f 7d67713
---


# The resolver

The mechanism, and the reason this system is not a supervisor.

You name a tool. The binary reads [[the-ledger]], walks that tool's dependency
chain to its far end, and spawns the **first** program the chain requires — not
the tool that was asked for. That program comes up and re-enters the binary for
the next tool along the chain. That one comes up and re-enters it again. The
chain assembles itself from the bottom, one re-entry per link, until the named
tool is running at the top of a tree that was resolved entirely at runtime, out
of whatever the ledger held at the moment of the ask.

The binary is therefore invoked once per node, and each invocation has exactly
one job: resolve a single step and hand off. Nothing sits above the tree owning
it. There is no long-lived coordinator to lose state, to restart, or to become
the thing every cartridge waits on.

The property that must hold, and must be demonstrable rather than asserted:
because a dependency *launches* its dependent, the process tree and the
dependency tree are the same tree. Nothing maintains that correspondence, so it
cannot drift. A node exits and everything that needed it goes with it — teardown
is correctly ordered for free, and uninstalling is the absence of a launch.

Pointers: `deps()` in `src/main.rs` already performs this walk for display, and
`src/loader.rs` already resolves an `inject` key to its providing cartridge. The
walk exists; this PRD makes it the launch path instead of a report.

Lookup is a walk, not a map hit. [[the-manifest]]'s namespacing fork is settled
— *hidden until passed on* — so a `need` resolves against the asking cartridge's
own subtree first and walks outward from there, and it binds only to keys that
were provided in that subtree or re-exported into it. A provide key one subtree
over is invisible and is not a candidate, however identical the name. The
`inject`-to-provider search in `src/loader.rs` searches a flat list today and is
the thing this changes.

Must not change: a cycle in the chain is still detected and still refused —
`deps()` tracks a stack for exactly this and the launch path inherits the duty.

At the end, starting any tool brings its whole chain up from the bottom, and
`ps` shows the dependency tree.

## History

**failed, retried 2026-09-12 19:54**

**2026-09-12 19:53 — the lane holds work the spec did not claim**

`lane/the-resolver` has 8 path(s) standing outside the footprint:

- `Cargo.toml`
- `src/cartridge.rs`
- `src/lib.rs`
- `src/main.rs`
- `src/resolver.rs`
- `src/tests/fixtures/chain_fixture.rs`
- `src/tests/mod.rs`
- `src/tests/resolver.rs`

Nothing merged and nothing lost: the paths stand in the lane. Widen the footprint in the spec or drop the files, then `pearde retry the-resolver` puts a worker back on that lane.

## Report

spec01: exit 0

running 12 tests
test tests::resolver::a_cycle_is_refused_not_walked ... ok
test tests::resolver::a_clash_is_refused_naming_every_offer ... ok
test tests::resolver::a_chain_walks_to_its_far_end_before_the_tool_it_names ... ok
test tests::resolver::a_nested_provider_answers_before_any_outer_one ... ok
test tests::resolver::an_unbound_key_refuses_the_launch ... ok
test tests::resolver::a_diamond_launches_its_shared_provider_once ... ok
test tests::resolver::a_hosted_node_runs_its_lua_component_and_provides_its_keys ... ok
test tests::resolver::a_reentry_into_an_uninstalled_node_launches_nothing ... ok
13637
test tests::resolver::the_launch_refuses_a_cycle_the_ledger_holds ... ok
test tests::resolver::a_chain_node_whose_binary_does_not_exist_refuses_the_ask ... ok
test tests::resolver::a_chain_of_lua_cartridges_is_hosted_and_the_cascade_follows_the_dependency ... ok
test tests::resolver::a_chain_comes_up_from_the_bottom_and_teardown_follows_the_dependency ... ok

test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 75 filtered out; finished in 0.76s

    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.02s
     Running unittests src/lib.rs (target/debug/deps/zirkle-9dda9247c73c08cf)

spec02: exit 0

running 12 tests
test tests::resolver::a_cycle_is_refused_not_walked ... ok
test tests::resolver::a_clash_is_refused_naming_every_offer ... ok
test tests::resolver::a_chain_walks_to_its_far_end_before_the_tool_it_names ... ok
test tests::resolver::an_unbound_key_refuses_the_launch ... ok
test tests::resolver::a_nested_provider_answers_before_any_outer_one ... ok
test tests::resolver::a_diamond_launches_its_shared_provider_once ... ok
test tests::resolver::a_hosted_node_runs_its_lua_component_and_provides_its_keys ... ok
test tests::resolver::a_reentry_into_an_uninstalled_node_launches_nothing ... ok
13761
test tests::resolver::a_chain_node_whose_binary_does_not_exist_refuses_the_ask ... ok
test tests::resolver::the_launch_refuses_a_cycle_the_ledger_holds ... ok
test tests::resolver::a_chain_of_lua_cartridges_is_hosted_and_the_cascade_follows_the_dependency ... ok
test tests::resolver::a_chain_comes_up_from_the_bottom_and_teardown_follows_the_dependency ... ok

test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 75 filtered out; finished in 0.72s

== build
root: /var/folders/_p/tzmzw3m10kg7sg9hc7_mkm7w0000gn/T/tmp.Q4KkOcjNKy
== ask: zirkle up tool.run
13845
== the tree (ps): one process per node, each dependent a child of its dependency
  db pid=13845 ppid=1
  store pid=13847 ppid=13845
  tool pid=13849 ppid=13847
== teardown: the far end is killed, its dependents go with it
  dependents still alive after the far end died: 0 (want 0)
== uninstalling is the absence of a launch
  refused: `db.key` asked for at `store` binds to nothing in scope
== a cycle is refused
  refused: `a.key` resolves into `a` again: cycle
== a Lua-entry chain is hosted: the binary itself is the program
14016
== the hosted tree (ps): one host process per node, each dependent a child of its dependency
  db pid=14016 ppid=1 (hosted)
  store pid=14018 ppid=14016 (hosted)
  tool pid=14019 ppid=14018 (hosted)
== teardown through the hosted chain: the far end is killed, its dependents go with it
  dependents still alive after the hosted far end died: 0 (want 0)
== pass one probe complete
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.02s
     Running unittests src/lib.rs (target/debug/deps/zirkle-9dda9247c73c08cf)

spec03: exit 0

running 12 tests
test tests::resolver::a_cycle_is_refused_not_walked ... ok
test tests::resolver::a_clash_is_refused_naming_every_offer ... ok
test tests::resolver::an_unbound_key_refuses_the_launch ... ok
test tests::resolver::a_diamond_launches_its_shared_provider_once ... ok
test tests::resolver::a_chain_walks_to_its_far_end_before_the_tool_it_names ... ok
test tests::resolver::a_nested_provider_answers_before_any_outer_one ... ok
test tests::resolver::a_hosted_node_runs_its_lua_component_and_provides_its_keys ... ok
test tests::resolver::a_reentry_into_an_uninstalled_node_launches_nothing ... ok
14090
test tests::resolver::the_launch_refuses_a_cycle_the_ledger_holds ... ok
test tests::resolver::a_chain_node_whose_binary_does_not_exist_refuses_the_ask ... ok
test tests::resolver::a_chain_of_lua_cartridges_is_hosted_and_the_cascade_follows_the_dependency ... ok
test tests::resolver::a_chain_comes_up_from_the_bottom_and_teardown_follows_the_dependency ... ok

test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 75 filtered out; finished in 0.63s


running 87 tests
test tests::bridge::process_scripts_resolve_relative_to_the_cartridge ... ok
test tests::bridge::bridge_manifest_rejects_paths_outside_the_cartridge ... ok
test tests::cartridges::config_lua_overrides_entries_and_manifest_resolves_providers ... ok
test tests::cartridges::a_glob_injects_every_matching_key_the_other_entries_provide ... ok
test tests::cartridges::a_changed_file_swaps_the_fiber_and_a_broken_one_keeps_it ... ok
test tests::bridge::bridge_status_tracks_active_generations_and_scopes_backend_calls ... ok
test tests::contracts::a_broken_integration_check_fails_closed_with_its_error ... ok
test tests::contracts::a_broken_selftest_fails_closed_and_names_the_cartridge ... ok
test tests::contracts::a_cartridge_declaring_nothing_is_never_called ... ok
test tests::contracts::declared_contracts_run_after_apply_and_pass ... ok
test tests::folders::malformed_manifests_fail_without_evaluating_entries ... ok
test tests::contracts::one_obligation_may_be_declared_alone ... ok
test tests::folders::folder_manifest_names_the_component_and_reloads_its_entry ... ok
test tests::ledger::a_root_that_does_not_exist_is_an_empty_ledger ... ok
test tests::ledger::a_new_cartridge_is_available_and_not_started ... ok
test tests::ledger::a_walk_answers_from_the_asker_subtree_then_steps_outward ... ok
test tests::ledger::a_walk_from_a_nested_scope_steps_outward ... ok
test tests::ledger::an_entry_is_its_path_from_the_root ... ok
test tests::ledger::an_unreadable_document_is_an_entry_with_its_reason ... ok
test tests::ledger::two_entries_of_one_scope_offering_one_key_is_a_clash ... ok
test tests::lifecycle::a_raising_step_fails_the_fiber_with_nothing_installed ... ok
test tests::foreground::foreground_call_returns_the_service_value_and_disposes_the_profile ... ok
test tests::foreground::foreground_missing_or_failed_service_still_disposes ... ok
test tests::lifecycle::a_cycle_stays_inactive ... ok
test tests::cartridges::yolo_overrides_cartridge_config_only_for_automatic_services ... ok
test tests::cartridges::reconcile_adds_removes_and_revises_by_id ... ok
test tests::cartridges::yields_are_boundaries_and_access_is_enforced ... ok
test tests::lifecycle::effects_revert_in_lifo_order ... ok
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
test tests::manifest::malformed_declarations_are_refused_without_evaluating_the_entry ... ok
test tests::manifest::probe_nesting_is_not_discovered ... ok
test tests::manifest::the_capability_request_is_on_the_same_document ... ok
test tests::manifest::the_document_declares_what_it_provides_and_needs ... ok
test tests::manifest::the_document_overrides_what_the_entry_declares ... ok
test tests::lifecycle::disposing_a_parent_retires_its_children ... ok
test tests::manifest::two_subtrees_may_provide_the_same_key ... ok
test tests::process::closed_links_release_pending_and_reject_new_requests ... ok
test tests::process::disabled_processes_never_run_hello_apply_or_replace ... ok
test tests::process::dropping_a_waiter_ignores_its_late_reply ... ok
test tests::process::invalid_registration_fails_before_changing_loaded_fibers ... ok
test tests::lifecycle::a_target_change_stops_the_iterator_at_the_boundary ... ok
test tests::lifecycle::isolated_realms_bind_independently ... ok
test tests::lifecycle::access_is_enforced_at_the_point_of_use ... ok
test tests::lifecycle::dependents_are_drained_before_any_provider_inverse ... ok
test tests::lifecycle::a_replaced_provider_reloads_its_dependents ... ok
test tests::reload::corrected_code_can_recover_a_failed_initial_generation ... ok
test tests::reload::switching_keeps_consumers_bound_and_rejects_bad_migrations ... ok
test tests::reload::two_entries_of_one_file_each_get_their_own_switch ... ok
test tests::process::sdk_eof_and_dispose_release_waiters_before_finalizers ... ok
test tests::process::sdk_child_roundtrip_errors_metadata_and_eof ... ok
test tests::process::ordinary_lua_composition_can_load_a_process_wrapper ... ok
test tests::resolver::a_chain_walks_to_its_far_end_before_the_tool_it_names ... ok
test tests::resolver::a_clash_is_refused_naming_every_offer ... ok
test tests::resolver::a_cycle_is_refused_not_walked ... ok
test tests::resolver::a_diamond_launches_its_shared_provider_once ... ok
test tests::process::lua_wrappers_merge_injections_before_start_and_preserve_config ... ok
test tests::resolver::a_nested_provider_answers_before_any_outer_one ... ok
test tests::resolver::a_hosted_node_runs_its_lua_component_and_provides_its_keys ... ok
test tests::resolver::an_unbound_key_refuses_the_launch ... ok
test tests::cartridges::wrapped_process_isolation_metadata_and_dependency_restart_compose ... ok
test tests::rpc_contract::rust_sdk_and_lua_share_json_and_bidirectional_contracts ... ok
test tests::rpc_contract::rust_sdk_eof_rejects_another_inflight_call ... ok
test tests::rpc_contract::rust_sdk_failed_apply_retains_generation_then_reload_and_dispose_work ... ok
test tests::socket::a_client_round_trips_through_a_cartridge ... ok
test tests::socket::one_turn_id_crosses_the_socket_the_host_a_cartridge_and_lua ... ok
test tests::folders::watcher_reloads_a_folder_when_its_manifest_changes ... ok
test turn::tests::a_credential_or_a_prompt_body_is_omitted_and_named ... ok
test tests::socket::socket_calls_a_lua_wrapped_sdk_process_with_correlated_errors ... ok
test turn::tests::the_sink_stays_bounded_by_rotating_one_generation ... ok
test tests::process::a_process_provides_listens_and_is_disposed ... ok
test tests::resolver::a_reentry_into_an_uninstalled_node_launches_nothing ... ok
test tests::resolver::the_launch_refuses_a_cycle_the_ledger_holds ... ok
test tests::cartridges::local_list_resolves_wrappers_without_starting_a_daemon_or_applying_cartridges ... ok
14283
test tests::resolver::a_chain_node_whose_binary_does_not_exist_refuses_the_ask ... ok
test tests::resolver::a_chain_of_lua_cartridges_is_hosted_and_the_cascade_follows_the_dependency ... ok
test tests::resolver::a_chain_comes_up_from_the_bottom_and_teardown_follows_the_dependency ... ok
test tests::process::watches_reload_wrappers_binaries_and_new_executable_directories_once ... ok

test result: ok. 87 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.76s

== build
root: /var/folders/_p/tzmzw3m10kg7sg9hc7_mkm7w0000gn/T/tmp.dNtahseZIR
== ask: zirkle up tool.run
14423
== the tree (ps): one process per node, each dependent a child of its dependency
  db pid=14423 ppid=1
  store pid=14425 ppid=14423
  tool pid=14426 ppid=14425
== teardown: the far end is killed, its dependents go with it
  dependents still alive after the far end died: 0 (want 0)
== uninstalling is the absence of a launch
  refused: `db.key` asked for at `store` binds to nothing in scope
== a cycle is refused
  refused: `a.key` resolves into `a` again: cycle
== a Lua-entry chain is hosted: the binary itself is the program
14483
== the hosted tree (ps): one host process per node, each dependent a child of its dependency
  db pid=14483 ppid=1 (hosted)
  store pid=14485 ppid=14483 (hosted)
  tool pid=14486 ppid=14485 (hosted)
== teardown through the hosted chain: the far end is killed, its dependents go with it
  dependents still alive after the hosted far end died: 0 (want 0)
== pass one probe complete
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.02s
     Running unittests src/lib.rs (target/debug/deps/zirkle-9dda9247c73c08cf)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.02s
     Running unittests src/lib.rs (target/debug/deps/zirkle-9dda9247c73c08cf)
{"msg":"syntax error: [string \"/private/var/folders/_p/tzmzw3m10kg7sg9hc7_mk...\"]:1: syntax error near 'is'","src":"p","t":1789235849711,"turn":null}
{"msg":"syntax error: [string \"/private/var/folders/_p/tzmzw3m10kg7sg9hc7_mk...\"]:1: syntax error near 'Lua'","src":"p","t":1789235849712,"turn":null}
{"msg":"runtime error: /private/var/folders/_p/tzmzw3m10kg7sg9hc7_mkm7w0000gn/T/.tmpp5NKb2/p/cartridge.json: expected value at line 1 column 1","src":"instance","t":1789235849718,"turn":null}
{"msg":"runtime error: [string \"/private/var/folders/_p/tzmzw3m10kg7sg9hc7_mk...\"]:3: migration rejected\nstack traceback:\n\t[C]: in function 'error'\n\t[string \"/private/var/folders/_p/tzmzw3m10kg7sg9hc7_mk...\"]:3: in function <[string \"/private/var/folders/_p/tzmzw3m10kg7sg9hc7_mk...\"]:1>","src":"p","t":1789235849857,"turn":null}
{"msg":"rejected fixture migration","src":"peer","t":1789235850196,"turn":null}
