---
kind: work
description: "The agent can retrieve an accurate map of its own program"
status: done
level: 10
priority: P1
estimate: 1d
---

# The agent can discover its own program

## Outcome

The agent can find the context needed to understand and change the program it
inhabits: architecture, active profile, cartridges, service and tool contracts,
source locations, language boundaries, state ownership, tests and reload rules.
A small entry memo leads to details on demand. The running inventory supplies
current facts; authored memos explain intent and procedure.

## Check

- [x] Starting with only the normal system context and the entry memo, an agent
      can locate the active profile and map each enabled tool/service to its
      provider, source or installed artifact, declared dependencies and tests.
- [x] Cartridge-owned memos expose usage, examples, debug probes and extension
      instructions. Enabled, disabled, missing and failed providers are clearly
      distinguished; unavailable source has an explicit retrieval reference.
- [x] Adding or reloading a fixture cartridge refreshes the discoverable inventory
      and its relevant memo context. Stale revisions are identifiable.
- [x] The bootstrap context links to the map instead of embedding source or every
      cartridge document. A fixture records bootstrap size and demonstrates
      targeted retrieval through the existing memo and shell tools.

## Approach

Use the profile/catalog, manifests, SDK contracts, repository guidance and
cartridge-owned records as sources. Keep generated runtime facts separate from
authored explanations. This delivers the context needed by debug and extension
work, without adding another model-facing tool.

## Spec

Probe committed on `work/program-map`: `9901797` adds actual-host inventory
through the existing memo/SDK connection; `aac05b6` adds bounded retrieval,
375-byte bootstrap map, source/contracts/check memos and reload tests.
Runtime facts include committed generations, dependencies and source revisions.

Remaining: include dynamic child providers and tracked source paths as inventory
rows, prove cartridge-specific context coverage across the default profile,
then integrate and run the full gate. Remaining estimate: 4h. The user requested
committing current work and merging completed scope into main; this unfinished
probe remains in `.claude/worktrees/program-map` with no active worker.

## Validation checkpoint

Inventory integration 2, memo 28 and policy 4 tests passed. Formatting, focused
Clippy and diff checks passed. No full integrated gate was run for this probe;
acceptance boxes stay open.

Box 1 (2026-09-12, lane work/program-map @ 55cf1b3). Inventory against the default profile with the lane build (`zirkle run memo {"op":"inventory","limit":20}`): profile <lane>/.zirkle/default, every entry mapped — agent Active provide=[agent] deps=(sessions→sessions, buffers→sessions, policy→policy, router→router, harness→harness, tool.shell→pty, tool.memo→memo) tracked=16 memos=True tests=True; harness Active deps=(environment→pty, pty→pty, clock→clock, router→router, memo→memo, sessions→sessions, buffers→sessions); pty Active provide=[pty, environment, tool.shell]; router, sessions, clock, policy Active; memory Failed err="process executable `memory` was not found"; ui Loading→Active deps=(agent, sessions, buffers, router, pty). The entry memo program-entry.md (11 lines) links the program-map note, which documents the op and row fields.

Box 2 (2026-09-12, lane work/program-map @ 55cf1b3). Default-profile inventory rows carry context.memos for memo, agent, harness, policy, sessions and ui (their shipped records), and context.source paths plus tracked lists for every row; the memory row is Failed with its explicit error, so unavailable is distinguishable from Active. Unit test inventory_distinguishes_provider_states_and_refreshes_committed_generations asserts Active, Inactive, Failed (error text surfaced), Missing, and Disabled rows and that disabled declarations are unavailable. The addon fixture manifest carries source=https://example.test/addon/revision-1 and inventory returns it as context.source_reference; a Lua wrapper without a reference gets implementation_source=null. Nextest: Summary 1337 tests run: 1337 passed.

Box 3 (2026-09-12, lane work/program-map @ 55cf1b3). memo_inventory_and_targeted_context_work_in_the_actual_host reconciles a fixture addon into the profile, its inventory row appears with a new generation, and after editing addon/init.lua and host.replace the row generation changes again; the fixture memo read returns a changed revision digest ("Use version two"); a cursor taken before the change is refused with "stale inventory cursor; restart inventory". inventory_lists_dynamic_children_and_tracked_source_paths (new) proves child-fiber rows and tracked source paths. Both pass under nextest (1337/1337).

Box 4 (2026-09-12, lane work/program-map @ 55cf1b3). The fixture test asserts the composed system bootstrap stays under 600 bytes ("bootstrap bytes: N" guard), contains the program-map link, and does NOT embed cartridge contracts (no "# Cartridge contracts" in the template); targeted retrieval goes through the existing memo tool (`read` of note/program-map.md returns a revision digest) and tool.memo — no new model-facing tool. Passing in memo_inventory_and_targeted_context_work_in_the_actual_host (nextest 1337/1337).

Full integrated gate (2026-09-12, lane work/program-map, `just all`): exit 0 — cargo fmt, clippy -D warnings, builtin/ui check+test, nextest 1337 passed / 17 skipped, memory workspace 224 passed, doc tests ok, tools and core python suites ok (core python run per file; test_development had failed once under an exported CARGO_TARGET_DIR that redirected the fixture build, seen failing then green on the same ZIRKLE_WRAPPER binary with the var unset). `just check` green earlier in the isolated target as well.

actual: committed 55cf1b3 on work/program-map in .claude/worktrees/program-map — inventory rows for dynamic child providers (dynamic, parent-named, dependency resolution names them) and per-entry tracked git source paths, shipped program-map note updated, new test inventory_lists_dynamic_children_and_tracked_source_paths; all four Check boxes ticked with evidence above; `just all` green; lane left unlanded for the coordinator.
