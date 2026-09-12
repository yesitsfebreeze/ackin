---
complexity: 10
footprint:
  - src/ledger.rs
  - src/tests/ledger.rs
  - src/lib.rs
  - src/tests/mod.rs
---

# spec01 — the ledger is a namespace of subtrees, derived off the root

The contract's data structure, settled by the namespacing decision in the
manifest contract: every cartridge installed under one root, keyed by its
`/`-joined path from that root and never by its bare name; a lookup is an
outward walk from the asker's own subtree to the root, not a flat map hit.

## What already stands

`src/ledger.rs` — `Ledger::scan(root)` reads the tree: a directory is a
cartridge exactly when it holds `cartridge.json`; the root's direct children
are top-level entries and a cartridge's direct children are its nested entries,
recursively. A directory that is not a cartridge is not descended into, so a
plain folder under the root does not extend a subtree. A root that does not
exist is an empty ledger, not an error. An unreadable document is an entry with
its reason (`unread`) and empty declarations. The ledger's read runs the
document's own checks *and* the host's re-export check (`passed_on`), so it
refuses exactly the trees the host refuses to load and the two listings can
never disagree about what reads.

`Installed::path` is the identity; `parent()` derives the scope;
`offers()` chains `provide` and `export`. `Ledger::resolve(from, key)` walks
`outward(from)` — the asker's own children, then its parent's, out to the root
— and returns `Bound<'a>`: `One(&Installed)`, `Clashed(Vec<&Installed>)` (two
or more entries of one scope offering the key, in path order — the answered
fork, sorting names the clash, it does not pick a winner), or `None`.
`bindings()` pairs every need of every entry with its `Bound`.

The walk contract, as the walk implements it: a nested cartridge's `provide`
is a candidate for every lookup whose walk passes its parent's scope, so it is
seen by everything inside the parent's subtree — the parent, the parent's other
children, their descendants — and invisible to the graph outside the parent
unless a parent passes it on. This is the settled reading of the namespacing
decision's "inner cartridges are hidden until passed on": hidden from outside
the parent, not from the siblings within it. The manifest contract's sentence
"satisfies its parent's needs and nothing else" names the graph outside the
parent, and holds as written on that side — a lookup at the root, or from any
unrelated subtree, cannot bind a nested key with no re-export anywhere.

`src/tests/ledger.rs` — six tests:
`an_entry_is_its_path_from_the_root`,
`a_walk_answers_from_the_asker_subtree_then_steps_outward`,
`two_entries_of_one_scope_offering_one_key_is_a_clash`,
`an_unreadable_document_is_an_entry_with_its_reason`,
`a_root_that_does_not_exist_is_an_empty_ledger`, and
`a_new_cartridge_is_available_and_not_started` (see spec02).

The probe — `.pearde/prds/the-ledger/probe/the-ledger-is-the-tree.sh` — proves
the same six things end to end against the built binary, fixture built at run
time in a temp directory. Green as of this pass.

## What is left

One lookup the walk handles but nothing has ever run: a request made *from a
nested scope*. Every probe and test asks from a top-level entry. The walk's
outward steps past the first are unexercised, and the asker-exclusion
(`e.path != from`) is only ever tested where the asker is also the nearest
offering scope.

## Acceptance

- [x] A lookup from a nested scope steps outward through more than one scope, proven by a test in `src/tests/ledger.rs` that fails without the walk.
  - Test `a_walk_from_a_nested_scope_steps_outward`, rebuilt in the skeptic
    corrections pass on a tree every valid document passes:
    `outer/inner/deep` declares `needs: ["outer.top"]`, the two scopes
    between it and the top are silent, and `outer` provides `outer.top` —
    `resolve("outer/inner/deep", "outer.top")` is `One("outer")`, and the same
    ask is pinned through `bindings()` as a declared need. Without
    `outward()` stepping past the first scope the test cannot pass; cut to the
    asker's own subtree it reads `?`. The shape first proven here — `outer`
    declaring `export: ["store.get"]` with a silent child, asked for by a key
    the asker itself provided — is a tree the document format refuses, and a
    lookup no valid document can make; the asker-exclusion it claimed to pin
    is pinned synthetically in
    `a_walk_answers_from_the_asker_subtree_then_steps_outward`.
- [x] `cargo test --offline tests::ledger::` passes with at least the six this pass left plus the one above, and every name it prints comes from `src/tests/ledger.rs`.
  - Baseline this pass: `test result: ok. 6 passed; 0 failed; 0 ignored; 0
    measured; 68 filtered out`. After the box above: 7.
  - mason 2026-09-12: `test result: ok. 7 passed; 0 failed; 0 ignored; 0
    measured; 68 filtered out` — all seven names from `src/tests/ledger.rs`.
- [x] The whole suite stays green, which is the check that no caller of `resolve` was left on the old `Option` return.
  - Baseline this pass: `test result: ok. 74 passed; 0 failed` (was 68 at pass
    one). `grep -rn 'resolve(' src/` shows no caller matching on `Option`
    outside `ledger.rs`; `main.rs` matches on `Bound`, `bindings()` carries it.
  - mason 2026-09-12: `cargo test --offline` — `test result: ok. 75 passed; 0
    failed; 0 ignored; 0 measured; 0 filtered out`; the `resolve(` grep shows
    only `ledger.rs` (definition, `bindings`), `loader.rs` (`loader::resolve`,
    a different function), `lua.rs` and `main.rs:292` (matches on `Bound`).
- [x] `cargo clippy --offline --all-targets` is clean with `warnings = "deny"` in force.
  - Clean this pass after `touch src/lib.rs`; the box is re-run whenever
    `ledger.rs` moves.
  - mason 2026-09-12: `Finished \`dev\` profile [unoptimized + debuginfo]
    target(s) in 1.21s`, no warnings, after `touch src/lib.rs`.

## Verify and Proof

```sh
cd /Users/feb/dev/cartridge/.pearde/.lanes/the-ledger
cargo test --offline tests::ledger::
grep -rn 'resolve(' src/ | grep -v tests/
touch src/lib.rs && cargo clippy --offline --all-targets
bash /Users/feb/dev/cartridge/.pearde/prds/the-ledger/probe/the-ledger-is-the-tree.sh
```