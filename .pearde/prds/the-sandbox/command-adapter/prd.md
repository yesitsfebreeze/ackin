---
state: done
origin: requested
priority: 60
complexity: 6
blast-radius: mid
workflow: probe-then-spec
actual: 0.01h
---


# command-adapter — A shared fail-closed command constructor carries manifest policy into synchronous and asynchronous launches without changing existing callers.

A shared fail-closed command constructor carries manifest policy into synchronous and asynchronous launches without changing existing callers.

## Implementation handoff

a shared fail-closed constructor for cartridge commands

Complexity 6; blast-radius mid. Establish the already-built command API without
changing launch callers. Preserve its direct wall tests and compile all targets.
Owns `src/lib.rs`, `src/sandbox.rs`, and initial seed `src/sandbox/linux.rs`.
Keep every actual process launch under OS policy, including eventual discovery.


Source snapshots and verified probes are listed in `../report.md` under Exact source handoff. Apply only this child's files from those patches; preserve unrelated changes.

Apply `/Users/feb/dev/cartridge/.pearde/prds/the-sandbox/probe/shared-adapter.patch` in the child lane. It carries the already-tested first pass. The parent lane remains a backup, not the child base.

## History

**failed, retried 2026-09-12 22:57**

**2026-09-12 22:57 — the lane holds work the spec did not claim**

`lane/the-sandbox-command-adapter` has 1 path(s) standing outside the footprint:

- `src/sandbox/`

Nothing merged and nothing lost: the paths stand in the lane. Widen the footprint in the spec or drop the files, then `pearde retry the-sandbox/command-adapter` puts a worker back on that lane.

## Report

spec01: exit 0

running 127 tests
test sandbox::tests::an_empty_command_is_refused_before_launch ... ok
test sandbox::tests::a_net_grant_turns_the_network_on_and_an_empty_one_leaves_it_off ... ok
test sandbox::tests::an_empty_grant_builds_no_allowance_beyond_the_runtime ... ok
test sandbox::tests::a_write_grant_names_the_canonicalized_path_and_implies_the_read ... ok
test sandbox::tests::an_exec_grant_that_resolves_builds_a_literal_and_one_that_does_not_builds_nothing ... ok
test sandbox::tests::a_script_names_its_interpreter ... ok
test tests::bridge::process_scripts_resolve_relative_to_the_cartridge ... ok
test tests::bridge::bridge_manifest_rejects_paths_outside_the_cartridge ... ok
test tests::cartridges::a_glob_injects_every_matching_key_the_other_entries_provide ... ok
test tests::cartridges::a_changed_file_swaps_the_fiber_and_a_broken_one_keeps_it ... ok
test tests::cartridges::config_lua_overrides_entries_and_manifest_resolves_providers ... ok
test tests::bridge::bridge_status_tracks_active_generations_and_scopes_backend_calls ... ok
test tests::contracts::a_broken_integration_check_fails_closed_with_its_error ... ok
test tests::contracts::a_broken_selftest_fails_closed_and_names_the_cartridge ... ok
test tests::contracts::a_cartridge_declaring_nothing_is_never_called ... ok
test tests::contracts::a_need_that_binds_to_nothing_is_named_instead_of_run ... ok
test tests::contracts::a_clashed_need_stops_the_run_before_it_loads ... ok
test tests::contracts::a_failing_contract_names_the_path_the_obligation_and_the_key ... ok
test tests::contracts::an_unknown_cartridge_is_named_rather_than_run ... ok
test tests::contracts::a_nested_cartridge_is_verified_by_its_path_from_the_root ... ok
test tests::contracts::declared_contracts_run_after_apply_and_pass ... ok
test tests::contracts::one_cartridge_is_verified_without_a_profile ... ok
test tests::contracts::the_providers_a_need_binds_to_are_reached_but_never_graded ... ok
test tests::contracts::one_obligation_may_be_declared_alone ... ok
test tests::folders::folder_manifest_names_the_component_and_reloads_its_entry ... ok
test tests::folders::malformed_manifests_fail_without_evaluating_entries ... ok
test tests::foreground::foreground_call_returns_the_service_value_and_disposes_the_profile ... ok
test tests::ledger::a_child_provide_is_seen_by_its_parent_subtree_and_not_by_the_graph_outside ... ok
test tests::ledger::a_dangling_re_export_is_unread_on_the_ledger_read_too ... ok
test tests::ledger::a_new_cartridge_is_available_and_not_started ... ok
test tests::ledger::a_root_that_does_not_exist_is_an_empty_ledger ... ok
test tests::ledger::a_walk_answers_from_the_asker_subtree_then_steps_outward ... ok
test tests::ledger::a_walk_from_a_nested_scope_steps_outward ... ok
test tests::ledger::an_entry_is_its_path_from_the_root ... ok
test tests::ledger::an_unreadable_document_is_an_entry_with_its_reason ... ok
test tests::ledger::two_entries_of_one_scope_offering_one_key_is_a_clash ... ok
test tests::foreground::foreground_missing_or_failed_service_still_disposes ... ok
test tests::lifecycle::a_raising_step_fails_the_fiber_with_nothing_installed ... ok
test tests::cartridges::yolo_overrides_cartridge_config_only_for_automatic_services ... ok
test tests::cartridges::reconcile_adds_removes_and_revises_by_id ... ok
test tests::cartridges::yields_are_boundaries_and_access_is_enforced ... ok
test tests::lifecycle::a_cycle_stays_inactive ... ok
test tests::lifecycle::disposing_a_parent_retires_its_children ... ok
test tests::lifecycle::effects_revert_in_lifo_order ... ok
test tests::lifecycle::a_target_change_stops_the_iterator_at_the_boundary ... ok
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
test tests::lifecycle::access_is_enforced_at_the_point_of_use ... ok
test tests::manifest::the_document_overrides_what_the_entry_declares ... ok
test tests::manifest::two_subtrees_may_provide_the_same_key ... ok
test tests::node::the_document_carries_the_cartridges_own_config ... ok
test tests::lifecycle::isolated_realms_bind_independently ... ok
test tests::process::closed_links_release_pending_and_reject_new_requests ... ok
test tests::process::disabled_processes_never_run_hello_apply_or_replace ... ok
test tests::process::dropping_a_waiter_ignores_its_late_reply ... ok
test tests::process::invalid_registration_fails_before_changing_loaded_fibers ... ok
test tests::lifecycle::dependents_are_drained_before_any_provider_inverse ... ok
test tests::lifecycle::a_replaced_provider_reloads_its_dependents ... ok
test sandbox::tests::the_asynchronous_spawn_enforces_the_same_empty_grant ... ok
test sandbox::tests::the_synchronous_command_enforces_the_empty_grant ... ok
test tests::folders::watcher_reloads_a_folder_when_its_manifest_changes ... ok
test tests::reload::a_composed_handle_reloads_its_node_again_after_the_first_swap ... ok
test tests::process::a_process_provides_listens_and_is_disposed ... ok
test tests::reload::a_nested_node_is_swapped_and_its_dependent_follows ... ok
test tests::reload::an_uncomposed_node_and_an_unknown_uid_refuse_the_ask ... ok
test tests::reload::corrected_code_can_recover_a_failed_initial_generation ... ok
test tests::reload::an_ask_at_a_retired_uid_is_refused_and_the_node_still_reloads ... ok
test tests::reload::switching_keeps_consumers_bound_and_rejects_bad_migrations ... ok
test tests::reload::two_entries_of_one_file_each_get_their_own_switch ... ok
test tests::process::sdk_eof_and_dispose_release_waiters_before_finalizers ... ok
test tests::process::sdk_child_roundtrip_errors_metadata_and_eof ... ok
test tests::resolver::a_chain_walks_to_its_far_end_before_the_tool_it_names ... ok
test tests::resolver::a_clash_is_refused_naming_every_offer ... ok
test tests::resolver::a_cycle_is_refused_not_walked ... ok
test tests::resolver::a_diamond_launches_its_shared_provider_once ... ok
test tests::process::ordinary_lua_composition_can_load_a_process_wrapper ... ok
test tests::resolver::a_nested_provider_answers_before_any_outer_one ... ok
test tests::resolver::a_hosted_node_runs_its_lua_component_and_provides_its_keys ... ok
test tests::resolver::an_unbound_key_refuses_the_launch ... ok
test tests::process::lua_wrappers_merge_injections_before_start_and_preserve_config ... ok
test tests::rpc_contract::rust_sdk_and_lua_share_json_and_bidirectional_contracts ... ok
test tests::rpc_contract::rust_sdk_eof_rejects_another_inflight_call ... ok
test tests::rpc_contract::rust_sdk_failed_apply_retains_generation_then_reload_and_dispose_work ... ok
test tests::socket::a_client_round_trips_through_a_cartridge ... ok
test tests::socket::one_turn_id_crosses_the_socket_the_host_a_cartridge_and_lua ... ok
test tests::socket::socket_calls_a_lua_wrapped_sdk_process_with_correlated_errors ... ok
test tests::stream::a_failing_listener_publishes_an_error_event_on_its_channel ... ok
test tests::stream::a_lua_cartridge_publishes_and_a_late_watcher_replays_the_log ... ok
test tests::stream::a_lua_cartridge_watches_a_channel_a_socket_client_publishes_to ... ok
test tests::stream::a_process_cartridge_publishes_and_watches_over_the_wire ... ok
test tests::stream::a_queue_nobody_reads_ends_at_the_next_publish ... ok
test tests::cartridges::wrapped_process_isolation_metadata_and_dependency_restart_compose ... ok
test tests::stream::a_subscriber_receives_everything_published_to_its_channel_in_order ... ok
test tests::stream::a_subscriber_that_crashed_and_came_back_ends_up_where_it_was ... ok
test tests::stream::concurrent_publishers_leave_a_gapless_ordered_log ... ok
test tests::stream::leaving_is_an_event_everyone_still_on_the_channel_sees ... ok
test tests::stream::publishing_without_a_listener_costs_one_append_and_keeps_the_log ... ok
test tests::stream::a_repeat_subscribe_spawns_no_second_pump ... ok
test tests::resolver::a_reentry_into_an_uninstalled_node_launches_nothing ... ok
test tests::cartridges::local_list_resolves_wrappers_without_starting_a_daemon_or_applying_cartridges ... ok
97658
{"nodes":["db","store","tool"],"socket":"/tmp/zirkle-d4145de7fd3ec498-feb.sock","up":"tool.run"}
test tests::resolver::a_chain_node_whose_binary_does_not_exist_refuses_the_ask ... ok
test tests::resolver::the_launch_refuses_a_cycle_the_ledger_holds ... ok
test tests::node::a_chain_node_calls_its_dependency_and_serves_its_dependents ... ok
test turn::tests::a_credential_or_a_prompt_body_is_omitted_and_named ... ok
test tests::resolver::a_chain_of_lua_cartridges_is_hosted_and_the_cascade_follows_the_dependency ... ok
test turn::tests::the_sink_stays_bounded_by_rotating_one_generation ... ok
test tests::wire::a_cartridge_hosts_a_cartridge_over_the_same_wire ... ok
test tests::resolver::a_chain_comes_up_from_the_bottom_and_teardown_follows_the_dependency ... ok
test tests::wire::a_daemon_reload_carries_down_to_the_child_that_declared_it ... ok
test tests::wire::a_child_key_the_sub_host_never_declared_stays_private ... ok
test tests::wire::a_child_that_never_becomes_ready_fails_the_spawn_where_it_failed ... ok
test tests::wire::a_nested_bridge_call_resolves_against_the_daemons_profile_grant ... ok
test tests::wire::a_child_that_declared_no_reload_still_answers_null ... ok
test tests::process::watches_reload_wrappers_binaries_and_new_executable_directories_once ... ok

test result: ok. 127 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.67s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 8 tests
test sandbox::tests::an_empty_command_is_refused_before_launch ... ok
test sandbox::tests::an_empty_grant_builds_no_allowance_beyond_the_runtime ... ok
test sandbox::tests::a_write_grant_names_the_canonicalized_path_and_implies_the_read ... ok
test sandbox::tests::a_net_grant_turns_the_network_on_and_an_empty_one_leaves_it_off ... ok
test sandbox::tests::an_exec_grant_that_resolves_builds_a_literal_and_one_that_does_not_builds_nothing ... ok
test sandbox::tests::a_script_names_its_interpreter ... ok
test sandbox::tests::the_asynchronous_spawn_enforces_the_same_empty_grant ... ok
test sandbox::tests::the_synchronous_command_enforces_the_empty_grant ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 119 filtered out; finished in 0.45s

   Compiling libc v0.2.189
   Compiling proc-macro2 v1.0.107
   Compiling unicode-ident v1.0.24
   Compiling quote v1.0.47
   Compiling cfg-if v1.0.4
   Compiling serde_core v1.0.229
   Compiling shlex v2.0.1
   Compiling find-msvc-tools v0.1.12
   Compiling autocfg v1.5.1
   Compiling memchr v2.8.3
   Compiling bitflags v2.13.2
   Compiling parking_lot_core v0.9.12
   Compiling num-traits v0.2.19
   Compiling cc v1.4.5
   Compiling pin-project-lite v0.2.17
   Compiling utf8parse v0.2.2
   Compiling typeid v1.0.3
   Compiling futures-core v0.3.34
   Compiling lua-src v551.0.2
   Compiling scopeguard v1.2.0
   Compiling futures-sink v0.3.34
   Compiling smallvec v1.16.1
   Compiling serde v1.0.229
   Compiling pkg-config v0.3.34
   Compiling futures-channel v0.3.34
   Compiling lock_api v0.4.14
   Compiling anstyle-parse v1.0.0
   Compiling zmij v1.0.23
   Compiling futures-task v0.3.34
   Compiling is_terminal_polyfill v1.70.2
   Compiling erased-serde v0.4.10
   Compiling futures-io v0.3.34
   Compiling slab v0.4.12
   Compiling anstyle-query v1.1.5
   Compiling anstyle v1.0.14
   Compiling which v8.0.6
   Compiling errno v0.3.14
   Compiling syn v3.0.5
   Compiling luajit-src v210.7.3+1ee778a
   Compiling colorchoice v1.0.5
   Compiling anstream v1.0.0
   Compiling mlua-sys v0.12.0
   Compiling parking_lot v0.12.5
   Compiling objc2-core-foundation v0.3.2
   Compiling notify v9.0.0-rc.5
   Compiling futures-macro v0.3.34
   Compiling serde_derive v1.0.229
   Compiling heck v0.5.0
   Compiling thiserror v2.0.20
   Compiling strsim v0.11.1
   Compiling ordered-float v2.10.1
   Compiling futures-util v0.3.34
   Compiling serde_json v1.0.151
   Compiling rustix v1.1.4
   Compiling getrandom v0.4.3
   Compiling clap_lex v1.1.0
   Compiling same-file v1.0.6
   Compiling clap_builder v4.6.6
   Compiling walkdir v2.5.0
   Compiling clap_derive v4.6.4
   Compiling objc2-core-services v0.3.2
   Compiling thiserror-impl v2.0.20
   Compiling tokio-macros v2.7.2
   Compiling bstr v1.13.1
   Compiling signal-hook-registry v1.4.8
   Compiling mio v1.2.3
   Compiling socket2 v0.6.5
   Compiling notify-types v2.1.0
   Compiling xxhash-rust v0.8.18
   Compiling bytes v1.12.1
   Compiling futures-executor v0.3.34
   Compiling either v1.18.0
   Compiling itoa v1.0.18
   Compiling rustc-hash v2.1.3
   Compiling log v0.4.34
   Compiling futures v0.3.34
   Compiling tokio v1.53.1
   Compiling clap v4.6.6
   Compiling once_cell v1.21.4
   Compiling fastrand v2.5.0
   Compiling serde-value v0.7.0
   Compiling tempfile v3.27.0
   Compiling mlua v0.12.1
   Compiling zirkle v0.1.0 (/Users/feb/dev/cartridge/.pearde/.sessions/s77605)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 8.66s
     Running unittests src/lib.rs (target/debug/deps/zirkle-0a1ab6a502910f9f)
{"msg":"syntax error: [string \"/private/var/folders/_p/tzmzw3m10kg7sg9hc7_mk...\"]:1: syntax error near 'is'","src":"p","t":1789246661624,"turn":null}
{"msg":"syntax error: [string \"/private/var/folders/_p/tzmzw3m10kg7sg9hc7_mk...\"]:1: syntax error near 'Lua'","src":"p","t":1789246661624,"turn":null}
{"msg":"runtime error: /private/var/folders/_p/tzmzw3m10kg7sg9hc7_mkm7w0000gn/T/.tmpJ1YHJL/p/cartridge.json: expected value at line 1 column 1","src":"instance","t":1789246661639,"turn":null}
{"msg":"no running node carries uid 2","src":"2","t":1789246662250,"turn":null}
{"msg":"runtime error: [string \"/private/var/folders/_p/tzmzw3m10kg7sg9hc7_mk...\"]:3: migration rejected\nstack traceback:\n\t[C]: in function 'error'\n\t[string \"/private/var/folders/_p/tzmzw3m10kg7sg9hc7_mk...\"]:3: in function <[string \"/private/var/folders/_p/tzmzw3m10kg7sg9hc7_mk...\"]:1>","src":"p","t":1789246662272,"turn":null}
{"msg":"rejected fixture migration","src":"peer","t":1789246662502,"turn":null}
{"msg":"child refused reload","src":"parent","t":1789246664164,"turn":null}
     Running unittests src/main.rs (target/debug/deps/zirkle-115d354c3026b045)
     Running unittests src/tests/fixtures/chain_fixture.rs (target/debug/examples/chain_fixture-e602166043d09af4)
     Running unittests src/tests/fixtures/lua_fixture.rs (target/debug/examples/lua_fixture-3d19b1bff05fb50e)
     Running unittests src/tests/fixtures/nested_fixture.rs (target/debug/examples/nested_fixture-c78a77447a4b4df1)
     Running unittests src/tests/fixtures/rpc_fixture.rs (target/debug/examples/rpc_fixture-7bcd1d5a6b46e080)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.04s
     Running unittests src/lib.rs (target/debug/deps/zirkle-0a1ab6a502910f9f)
