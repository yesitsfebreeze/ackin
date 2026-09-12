---
state: done
origin: requested
priority: 70
complexity: 21
blast-radius: mid
needs:
  - the-wire
workflow: wire-the-chain
actual: 0.01h
commit: 560f762 7367355
---

# Lua interface

Lua stays. The user has confirmed it directly: it is the composition and
interface layer and it is not deleted.

It is how something is wired in and tried before it has earned a compiled
implementation — the interface code that makes a cartridge slottable while it is
still being vibed into shape. `mlua` is already a dependency and `src/lua.rs`,
`src/context.rs` and `src/loader.rs` already carry the host bindings, the `ctx`
surface and the entry protocol.

What this PRD settles is the surface a cartridge author writes against now that
the binary re-enters per node rather than composing a profile up front. The old
shape assumed a profile `init.lua` listing entries into one graph. The new shape
starts from a named tool and [[the-ledger]], which is a different thing to write
against even though it runs the same interpreter.

At the end, a new cartridge can be written, wired and run without touching Rust.

## Report

spec01: exit 0

running 2 tests
test tests::node::the_document_carries_the_cartridges_own_config ... ok
test tests::node::a_chain_node_calls_its_dependency_and_serves_its_dependents ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 95 filtered out; finished in 0.51s


running 97 tests
test tests::bridge::process_scripts_resolve_relative_to_the_cartridge ... ok
test tests::bridge::bridge_manifest_rejects_paths_outside_the_cartridge ... ok
test tests::cartridges::config_lua_overrides_entries_and_manifest_resolves_providers ... ok
test tests::cartridges::a_changed_file_swaps_the_fiber_and_a_broken_one_keeps_it ... ok
test tests::cartridges::a_glob_injects_every_matching_key_the_other_entries_provide ... ok
test tests::bridge::bridge_status_tracks_active_generations_and_scopes_backend_calls ... ok
test tests::contracts::a_broken_integration_check_fails_closed_with_its_error ... ok
test tests::contracts::declared_contracts_run_after_apply_and_pass ... ok
test tests::contracts::a_broken_selftest_fails_closed_and_names_the_cartridge ... ok
test tests::contracts::a_cartridge_declaring_nothing_is_never_called ... ok
test tests::folders::folder_manifest_names_the_component_and_reloads_its_entry ... ok
test tests::contracts::one_obligation_may_be_declared_alone ... ok
test tests::folders::malformed_manifests_fail_without_evaluating_entries ... ok
test tests::ledger::a_child_provide_is_seen_by_its_parent_subtree_and_not_by_the_graph_outside ... ok
test tests::ledger::a_dangling_re_export_is_unread_on_the_ledger_read_too ... ok
test tests::ledger::a_root_that_does_not_exist_is_an_empty_ledger ... ok
test tests::ledger::a_new_cartridge_is_available_and_not_started ... ok
test tests::ledger::a_walk_answers_from_the_asker_subtree_then_steps_outward ... ok
test tests::ledger::a_walk_from_a_nested_scope_steps_outward ... ok
test tests::ledger::an_unreadable_document_is_an_entry_with_its_reason ... ok
test tests::ledger::an_entry_is_its_path_from_the_root ... ok
test tests::ledger::two_entries_of_one_scope_offering_one_key_is_a_clash ... ok
test tests::lifecycle::a_raising_step_fails_the_fiber_with_nothing_installed ... ok
test tests::foreground::foreground_call_returns_the_service_value_and_disposes_the_profile ... ok
test tests::foreground::foreground_missing_or_failed_service_still_disposes ... ok
test tests::cartridges::yolo_overrides_cartridge_config_only_for_automatic_services ... ok
test tests::lifecycle::a_cycle_stays_inactive ... ok
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
test tests::manifest::two_subtrees_may_provide_the_same_key ... ok
test tests::lifecycle::disposing_a_parent_retires_its_children ... ok
test tests::node::the_document_carries_the_cartridges_own_config ... ok
test tests::lifecycle::a_target_change_stops_the_iterator_at_the_boundary ... ok
test tests::process::closed_links_release_pending_and_reject_new_requests ... ok
test tests::process::disabled_processes_never_run_hello_apply_or_replace ... ok
test tests::process::dropping_a_waiter_ignores_its_late_reply ... ok
test tests::process::invalid_registration_fails_before_changing_loaded_fibers ... ok
test tests::lifecycle::isolated_realms_bind_independently ... ok
test tests::lifecycle::access_is_enforced_at_the_point_of_use ... ok
test tests::lifecycle::dependents_are_drained_before_any_provider_inverse ... ok
test tests::lifecycle::a_replaced_provider_reloads_its_dependents ... ok
test tests::process::sdk_eof_and_dispose_release_waiters_before_finalizers ... ok
test tests::reload::corrected_code_can_recover_a_failed_initial_generation ... ok
test tests::reload::switching_keeps_consumers_bound_and_rejects_bad_migrations ... ok
test tests::reload::two_entries_of_one_file_each_get_their_own_switch ... ok
test tests::process::sdk_child_roundtrip_errors_metadata_and_eof ... ok
test tests::process::ordinary_lua_composition_can_load_a_process_wrapper ... ok
test tests::process::lua_wrappers_merge_injections_before_start_and_preserve_config ... ok
test tests::resolver::a_chain_walks_to_its_far_end_before_the_tool_it_names ... ok
test tests::resolver::a_clash_is_refused_naming_every_offer ... ok
test tests::resolver::a_cycle_is_refused_not_walked ... ok
test tests::resolver::a_diamond_launches_its_shared_provider_once ... ok
test tests::resolver::a_hosted_node_runs_its_lua_component_and_provides_its_keys ... ok
test tests::resolver::a_nested_provider_answers_before_any_outer_one ... ok
test tests::cartridges::wrapped_process_isolation_metadata_and_dependency_restart_compose ... ok
test tests::resolver::an_unbound_key_refuses_the_launch ... ok
test tests::folders::watcher_reloads_a_folder_when_its_manifest_changes ... ok
test tests::rpc_contract::rust_sdk_and_lua_share_json_and_bidirectional_contracts ... ok
test tests::rpc_contract::rust_sdk_eof_rejects_another_inflight_call ... ok
test tests::rpc_contract::rust_sdk_failed_apply_retains_generation_then_reload_and_dispose_work ... ok
test tests::socket::a_client_round_trips_through_a_cartridge ... ok
test tests::socket::one_turn_id_crosses_the_socket_the_host_a_cartridge_and_lua ... ok
test tests::socket::socket_calls_a_lua_wrapped_sdk_process_with_correlated_errors ... ok
test tests::process::a_process_provides_listens_and_is_disposed ... ok
test tests::resolver::a_reentry_into_an_uninstalled_node_launches_nothing ... ok
43283
{"nodes":["db","store","tool"],"socket":"/tmp/zirkle-320682527cda6ae6-feb.sock","up":"tool.run"}
test tests::resolver::the_launch_refuses_a_cycle_the_ledger_holds ... ok
test tests::cartridges::local_list_resolves_wrappers_without_starting_a_daemon_or_applying_cartridges ... ok
test tests::resolver::a_chain_node_whose_binary_does_not_exist_refuses_the_ask ... ok
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

test result: ok. 97 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.45s

== build
root: /var/folders/_p/tzmzw3m10kg7sg9hc7_mkm7w0000gn/T/tmp.ypDOt1BVB0
== write: six files, no Rust, no profile
== wire: the ledger reads the tree
echo  provides echo
tool  provides tool.hello
  echo <- echo
tool/child  provides child.note
  tool.hello <- tool
== run: one ask, the chain up from the bottom
43579
{"nodes":["echo","tool"],"socket":"/tmp/zirkle-fd6d121b129220dc-feb.sock","up":"tool.hello"}
node echo pid 43579
node tool pid 43583
== call tool.hello: the tool's handler calls its need, one node down
{"data": {"from": "echo", "said": "hello", "via": "document"}, "reply": 1}
== call child.note: the composed cartridge answers, reaching up through the tool
{"data": {"from": "echo", "said": "nested", "via": "child"}, "reply": 1}
== call nowhere: a key nothing provides is refused by name
{"error": "`nowhere` is not provided", "reply": 1}
== kill the dependency: the dependent follows
the tool went with its dependency (pid 43583 gone)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.05s
     Running unittests src/lib.rs (target/debug/deps/zirkle-9dda9247c73c08cf)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.03s
     Running unittests src/lib.rs (target/debug/deps/zirkle-9dda9247c73c08cf)
{"msg":"syntax error: [string \"/private/var/folders/_p/tzmzw3m10kg7sg9hc7_mk...\"]:1: syntax error near 'is'","src":"p","t":1789237468294,"turn":null}
{"msg":"syntax error: [string \"/private/var/folders/_p/tzmzw3m10kg7sg9hc7_mk...\"]:1: syntax error near 'Lua'","src":"p","t":1789237468295,"turn":null}
{"msg":"runtime error: /private/var/folders/_p/tzmzw3m10kg7sg9hc7_mkm7w0000gn/T/.tmpMpX0j7/p/cartridge.json: expected value at line 1 column 1","src":"instance","t":1789237468300,"turn":null}
{"msg":"runtime error: [string \"/private/var/folders/_p/tzmzw3m10kg7sg9hc7_mk...\"]:3: migration rejected\nstack traceback:\n\t[C]: in function 'error'\n\t[string \"/private/var/folders/_p/tzmzw3m10kg7sg9hc7_mk...\"]:3: in function <[string \"/private/var/folders/_p/tzmzw3m10kg7sg9hc7_mk...\"]:1>","src":"p","t":1789237468544,"turn":null}
{"msg":"rejected fixture migration","src":"peer","t":1789237468845,"turn":null}
{"msg":"child refused reload","src":"parent","t":1789237470244,"turn":null}

spec02: exit 0

running 1 test
test tests::node::the_document_carries_the_cartridges_own_config ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 96 filtered out; finished in 0.00s


running 15 tests
test tests::manifest::a_blank_grant_path_is_refused_like_a_blank_key ... ok
test tests::manifest::an_empty_document_requests_nothing ... ok
test tests::manifest::an_export_that_names_nothing_inside_is_refused ... ok
test tests::manifest::a_missing_file_the_document_names_does_not_blank_its_declarations ... ok
test tests::manifest::an_unreadable_document_asks_for_nothing_knowable_not_for_nothing ... ok
test tests::manifest::malformed_declarations_are_refused_without_evaluating_the_entry ... ok
test tests::manifest::the_capability_request_is_on_the_same_document ... ok
test tests::manifest::a_parent_passes_an_inner_key_outward_by_naming_it ... ok
test tests::manifest::a_cartridge_two_levels_down_is_not_offered ... ok
test tests::manifest::a_disabled_cartridge_still_declares_what_it_asked_for ... ok
test tests::manifest::a_grandchild_key_reaches_the_top_only_through_every_level ... ok
test tests::manifest::the_document_declares_what_it_provides_and_needs ... ok
test tests::manifest::the_document_overrides_what_the_entry_declares ... ok
test tests::manifest::probe_nesting_is_not_discovered ... ok
test tests::manifest::two_subtrees_may_provide_the_same_key ... ok

test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 82 filtered out; finished in 0.01s

    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.02s
     Running unittests src/lib.rs (target/debug/deps/zirkle-9dda9247c73c08cf)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.02s
     Running unittests src/lib.rs (target/debug/deps/zirkle-9dda9247c73c08cf)
