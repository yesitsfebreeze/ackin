Verdict: DONE

# hot-reload — implementer pass, as engineer (tess, 2026-09-12)

The subtree transaction (spec01) stood in the lane from the pass-one build and
is re-verified box by box below, with one box that had no test yet — a node
composed without a rebuild, and an unknown uid — now covered by a new test. The
run's defined work was spec02: the Lua surface addressing the node, not the
generation.

## What moved (spec02)

- `src/reload.rs` — `Reload::same`: two handles name the same transaction when
  the shared parts are the same allocation (`Arc::ptr_eq` on gate and pending).
- `src/context.rs` — `LuaFiber` now carries the node's shared `Reload`
  transaction beside the generation handle it was minted with (captured at
  composition, before the first swap can retire that uid); `reload` fires
  through `Host::request_reload_for` instead of the raw uid.
- `src/loader.rs` — `Host::request_reload_for(uid, reload)`: a live uid keeps
  the existing `request_reload` dispatch unchanged; a uid the tree retired is
  re-anchored to the unstaged, unretired fiber carrying the same transaction;
  a transaction no live fiber carries is refused on the report channel, as the
  raw ask's refusals already are.
- `src/tests/reload.rs` — two spec02 tests
  (`a_composed_handle_reloads_its_node_again_after_the_first_swap`,
  `an_ask_at_a_retired_uid_is_refused_and_the_node_still_reloads`) and
  spec01's missing box-4 test
  (`an_uncomposed_node_and_an_unknown_uid_refuse_the_ask`), plus a
  `version_of` poll helper so the tests hold no fixed swap sleep.

Design note for the next worker: the identity must ride in the handle. A uid
names one generation; when a swap publishes, the old generation is retired and
removed from the registry, so a handle holding only a uid cannot even read the
transaction it once named — by then there is nothing left in the tree to look
up. That is why `request_reload_for` takes the `Reload` rather than
re-deriving it from the uid. (Refines the pass-one finding
`[[260912-61e6]]`.)

## Spec boxes and verify output

spec01 (`.pearde/prds/hot-reload/specs/spec01.md`) — 4/4 ticked, output quoted
in the spec.

```
cargo test --quiet --lib a_nested_node_is_swapped
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 100 filtered out; finished in 0.06s

cargo test --quiet --lib an_uncomposed_node      (added for box 4)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 100 filtered out; finished in 0.00s
```

spec02 (`.pearde/prds/hot-reload/specs/spec02.md`) — 3/3 ticked, output quoted
in the spec.

```
cargo test --quiet --lib a_composed_handle_reloads
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 100 filtered out; finished in 0.07s

cargo test --quiet --lib an_ask_at_a_retired_uid
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 100 filtered out; finished in 0.07s

cargo test --quiet --lib reload                  (the spec's verify block)
test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 87 filtered out; finished in 4.30s
```

## Repo gate

Full lane suite, `warnings = "deny"` in force (`[lints.rust]` in Cargo.toml):

```
cargo test --quiet        (in /Users/feb/dev/cartridge/.pearde/.lanes/hot-reload)
test result: ok. 101 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.33s
```

## Must-not-change held

`host.on_reload(true/false)` and the exclusive-resource preparation rule were
not touched: no file outside the four footprint files plus `src/reload.rs`
(the transaction's own identity method) moved, and `replace_entry`,
`swap_node` and the wire/ledger/resolver files are untouched. `git diff --stat`
in the lane:

```
src/context.rs       |  7 ++++++-
src/loader.rs        | 36 ++++++++++++++++++++++++++++++++++++
src/reload.rs        |  6 ++++++
src/tests/reload.rs  | 137 ++++++++++++++++++++++++++++++++++++++++++++
```

## Health floor

No file under the floor in this footprint. Everything that moved is inside the
footprint and spec-directed; nothing was left worse than found.

## Defects and gaps outside this scope

1. `resources/knowledge.py` does not exist in this repo (the brief names
   `python3 resources/knowledge.py query|remember|conclude`; neither
   `resources/` nor the script is present anywhere under `.pearde/`). The
   finding above is on record in the spec files and here; the wiki
   (`.pearde/wiki/sources/`) is the board's standing record and a later worker
   with the tool — or the orchestrator — should carry it there.
2. The watcher tracks sources per profile slot, so an edited nested source
   does not auto-reload its composed node — reload is asked for (carried over
   from pass one, unchanged by this run).
3. The SDK's `request_reload` (`src/sdk.rs:88`) has no fixture exercising the
   socket ask end to end; the host-side dispatch it lands on is covered
   (`an_ask_at_a_retired_uid`'s entry leg). A process fixture that fires
   `{"reload": true}` would close that gap; it sits outside this footprint.

## Prior pass report

The pass-one (analyst) report that stood here is preserved below.

---

Verdict: SPECCED

# hot-reload — pass one, as engineer

The transaction was already in the tree (`loader.rs` `replace_entry`,
`fiber.rs` `switch`, `reload.rs`, `service.rs`) and survives untouched. The
build added the subtree scope: `Host::replace_node(uid)` — a profile entry's
uid dispatches through `replace_entry`; a node composed inside another
cartridge runs the same transaction through a `rebuild` the composition
carries. Probe: `src/tests/reload.rs::a_nested_node_is_swapped_and_its_dependent_follows`
— an inner cartridge rebuilt from edited source and swapped under the running
tree, the composing fiber following through its stable service, a sibling
entry untouched, a raising candidate rejected with the live generation kept
and the transaction finished, and the same ask answered for a profile entry.
Full lane suite: 76 passed, 0 failed. Probe code left uncommitted in the lane
(`src/runtime.rs`, `src/fiber.rs`, `src/loader.rs`, `src/context.rs`,
`src/tests/reload.rs`).

## Specs

- `specs/spec01.md` — the subtree transaction (`replace_node`/`swap_node`,
  `Rebuild` on `Component`/`Fiber`, `Runtime::handle`/`ctx_under`). Stands
  built in the lane; boxes are re-verification. complexity 8.
- `specs/spec02.md` — the handle that survives its own swap: `LuaFiber`
  addressing the node by its shared reload identity instead of the generation
  uid, so a second `fiber:reload()` lands. Stands half-built; the rest is
  defined work. complexity 5.

## Scores

complexity: 13
blast-radius: mid
workflow: swap-the-node

complexity 13: the transaction and the probe test stand in the lane; what is
left defined is spec02's node addressing and implementer re-verification, so
the PRD's weight is the two small units, not the transaction.

blast-radius mid: the edits sit in the runtime's reload and composition files
(`loader.rs`, `runtime.rs`, `fiber.rs`, `context.rs`), which every
tree-shaping PRD reads, but the proven transaction was not rewritten and no
wire, ledger or resolver file is touched.

Footprint union: `.pearde/.lanes/hot-reload/src/loader.rs`,
`.pearde/.lanes/hot-reload/src/runtime.rs`,
`.pearde/.lanes/hot-reload/src/fiber.rs`,
`.pearde/.lanes/hot-reload/src/context.rs`,
`.pearde/.lanes/hot-reload/src/tests/reload.rs`.

## What the build hit

1. **A uid names a generation, not the node.** `switch` publishes the
   candidate under a new uid and retires the old fiber, so a handle captured
   before a swap is stale afterwards. What persists is the node's shared
   `Reload` transaction (every generation is composed with the same one) and,
   for a composed node, its `rebuild` — spec02 is the unit that makes the Lua
   surface use that. On record as [[260912-61e6]].
2. **A replaceable node must be resident.** `switch` refuses non-resident
   bindings ("replacement requires resident services"), so `ctx:cartridge`
   now composes resident and carries the rebuild; `swap_node` re-attaches both
   to every generation it builds, which the first generation alone does not.
3. **Rust-composed nested fibers are not replaceable.** Only Lua
   `ctx:cartridge` compositions carry a rebuild today; a node composed
   directly through `Ctx::cartridge` from Rust refuses `replace_node` with a
   named error (spec01's fourth box). Widening that is out of this contract.

Findings outside the scope: the watcher tracks sources per profile slot, so an
edited nested source does not auto-reload its composed node — reload is asked
for, which reads consistent with the contract but is worth a line in a later
pass. `pearde specced` was run to validate the set and flipped
`hot-reload: analyzing → specced` in the same call — the state on disk is
already specced, so the orchestrator's collect should expect that. The
specced check still warns that the verify blocks name no path under the
footprint (the sibling PRDs' specs carry the same warning); the blocks name
the lane paths in a trailing comment.
