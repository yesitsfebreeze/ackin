---
kind: work
description: "Context gets the current date without a mandatory clock cartridge"
status: done
level: 10
priority: P2
estimate: 1h
---

# Context gets the current date without a mandatory clock cartridge

## Outcome

The default and proxy harnesses obtain a fresh UTC date without mounting a standalone clock cartridge. Preserve deterministic time injection for tests and explicitly configured providers. This removes a required service round-trip for ordinary environment context.

## Spec

Files: `builtin/harness/main.rs`, `builtin/harness/tests/process.rs`, `core/tests/profile.rs`, `.zirkle/default/init.lua`, `.zirkle/proxy/init.lua`.

1. `builtin/harness/main.rs`: `Config.clock` defaults to `""`. `resolve_date` computes `utc_date()` natively when the key is empty — civil-from-days over `SystemTime` at call time, never frozen at startup — and calls the configured service key otherwise. Drop `"clock"` from the harness binary static `.inject` list; a profile that sets `clock = "key"` must also inject that key on the harness entry (entry injects merge beside the static ones).
2. Remove `{ id = "clock", path = "clock" }` from `.zirkle/default/init.lua` and `.zirkle/proxy/init.lua`. `builtin/clock` stays for explicit opt-in.
3. `builtin/harness/tests/process.rs`: the projection test configures `clock = "clock"` and pushes the `clock` injection so the fake date (`2026-09-09`) and the error/number modes still exercise the service route; the compaction tests run the native route (no clock configured).
4. `core/tests/profile.rs`: drop the clock fixture cartridge, adjust the manifest id/provide/inject indices after the entry leaves, and assert the substituted date only by shape (`FIRST 20...`).
5. Gates: `just check`, `just test`, `just all`.

The probe did all five steps; the work is committed on the lane (`work/context-date-needs-no-clock-cartridge` f8cb108) with every gate green. Left for the implementer: re-run the gates on the finished lane and land.

## Check

- [x] Both shipped profiles compose context with no clock entry or required clock service. `zirkle list` on the lane default profile: no clock entry; harness needs `sessions, buffers, router, memo, environment, pty` only. Same for `--profile proxy`: entries memo/sessions/router/policy/harness/pty/memory/proxy, no clock, no wait.
- [x] A fixed test clock covers the UTC day boundary and repeated requests after midnight; date resolution is not frozen at startup. `cargo test -p harness --bin harness native_date_covers_the_utc_day_boundary` -> `test tests::native_date_covers_the_utc_day_boundary_and_stays_fresh ... ok` (epoch 1970-01-01/02, month and leap boundaries, fresh `utc_date()` shape); `cargo test -p zirkle --lib native_system_memos_drive_live_harness_projection` -> `ok` (second request re-resolves the date natively, no clock service mounted).
- [x] Explicit custom time configuration either remains supported or has a documented migration with a focused compatibility test. Supported, with the focused compatibility test: `cargo test -p harness --test process harness_process_projects_context` -> `test harness_process_projects_context_over_the_real_rpc_path ... ok` — the test configures `clock = "clock"`, injects the key, and drives the fake date 2026-09-09 plus the error and number modes through the service route; `Config.clock` documents that a profile naming a key must inject it.
- [x] Gates: `just check` exit 0 (fmt, clippy -D warnings, bun tsc); `just test` and `just all` green — final `just all` run: exit 0, memory nextest 1337 pass, workspace nextest 224 pass, `test_router.py` `Ran 13 tests ... OK` (the launch tests included), all core python files OK, no FAILED in the log.

```sh
just check && just test && just all
```

Audit: [[runtime-audit-2026-09-12]].

## Actual

Landed in the lane `work/context-date-needs-no-clock-cartridge` (worktree `.claude/worktrees/context-date-needs-no-clock-cartridge`, commit f8cb108, not yet landed — left to the coordinator). One caveat for the record: two earlier full `just test` runs failed on `test_two_launches_at_once_get_their_own_ports` with `harness inactive waiting for clock` while other lanes ran gates concurrently; the cause is the shared cargo target (`/Users/feb/dev/sys/target`) — a foreign lane momentarily supplied a harness binary that still injects `clock`. The lane source and lane-built binary contain no clock inject (`harness hello` -> `{"inject":["sessions","buffers","router","memo"],"provide":["harness"],"reload":true}`), the isolated test and exact manual replication pass on the same binary, and the final uncontended `just all` is fully green. A private `CARGO_TARGET_DIR` attempt was abandoned because the volume hit 100% (149Mi free), filled by other lanes' private targets (`/private/tmp/program-map-target` 6.9G, `zirkle-native-profile-target` 2.8G) — the board should serialize gate runs or free disk.
