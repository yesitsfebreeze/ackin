---
complexity: 10
footprint:
  - src/loader.rs
  - src/tests/manifest.rs
  - src/tests/mod.rs
---

# spec02 — a name travels outward only where a parent passes it on

The answered fork makes a cartridge's `provide` keys private to its own
subtree: a nested cartridge satisfies its parent and nothing else, and an inner
name reaches the outside only where the parent names it in its own document.
This unit is the half of that rule the document can enforce by itself — that a
re-export names something the subtree actually offers, and that two subtrees
may claim the same key without colliding.

## What already stands

`Cartridge::offered` reads the cartridge's own folder, takes every direct child
directory holding a `cartridge.json`, and collects that child's `provide` plus
that child's own `export`. One level, deliberately: a grandchild's subtree is
hidden from the grandparent exactly as a child's is hidden from the outside, so
a deep key reaches the top only when every level between re-exports it.

`Cartridge::passed_on` runs that list against the document's `export` and
refuses any key nothing inside offers. It is called from `loader::resolve`,
before the Lua entry is evaluated, so a bad re-export is a read-time error —
and from `loader::document`, the narrower read the listing uses, so a
re-export is checked even for a cartridge nothing is trying to load.

`src/tests/manifest.rs` holds the three tests of the rule:
`two_subtrees_may_provide_the_same_key`,
`a_parent_passes_an_inner_key_outward_by_naming_it`,
`an_export_that_names_nothing_inside_is_refused`, and
`a_grandchild_key_reaches_the_top_only_through_every_level`, which asserts both
directions — refused while `inner` stays silent, accepted once `inner` passes
the key on.

`probe_nesting_is_not_discovered` marks the frontier on purpose: nothing loads
a nested cartridge, so the subtree is a property of the document and not yet of
the graph. That is [[the-ledger]]'s scan and [[the-resolver]]'s outward walk,
and this unit must not grow into either.

## What is left

`Cartridge::offered` decides, without the contract having said so, that a
nested cartridge is a direct child directory of its parent's folder. Nothing
else in the tree states that layout and no test pins it. Write the rule into
the code as a named constant or a documented helper, and add a test that a
`cartridge.json` one level deeper — `outer/vendor/inner/cartridge.json` — is
**not** offered to `outer`, so the one-level walk is a checked property rather
than an implementation accident.

`passed_on` is reached from `loader::resolve` only. The bundler path at
`src/loader.rs:695` calls `Cartridge::read` directly and never checks exports.
Confirm that is intended for bundling, and leave a comment saying so, or route
it through the same check.

## Acceptance

- [x] A `cartridge.json` two directories below a parent is not offered to that parent, proved by a test that fails when `offered` is made recursive.
  - `a_cartridge_two_levels_down_is_not_offered`: `outer/vendor/inner` provides
    `inner.key` and `vendor` is a plain directory, so
    `Cartridge::offered(outer)` is empty while
    `Cartridge::offered(outer/vendor)` is `["inner.key"]`, and `outer`
    re-exporting the key is refused.
  - Falsified: with `nested()` made recursive,
    `test result: FAILED. 13 passed; 2 failed` —
    `a_cartridge_two_levels_down_is_not_offered` and, with it,
    `a_grandchild_key_reaches_the_top_only_through_every_level`, whose first
    half asserts the same one-level rule from the inside. Reverted, 15 pass.
- [x] The direct-child layout rule is stated in `src/loader.rs` at the point `offered` applies it.
  - The walk moved out of `offered` into `fn nested(root: &Path)` directly
    above it, whose doc comment carries the rule: "**a nested cartridge is a
    direct child directory of its parent's folder holding a `cartridge.json`.**
    One level and no deeper", with `outer/vendor/inner` named as the case that
    is hidden. The file name itself is now `pub const MANIFEST`, used by
    `nested`, `classify` and `contracts` alike.
- [x] `an_export_that_names_nothing_inside_is_refused` fails when `passed_on`'s call in `loader::resolve` is removed.
  - With the call removed from both readers of the document —
    `loader::resolve` and `loader::document` —
    `test tests::manifest::an_export_that_names_nothing_inside_is_refused ... FAILED`,
    and with it three more: `a_cartridge_two_levels_down_is_not_offered`,
    `a_grandchild_key_reaches_the_top_only_through_every_level`,
    `an_unreadable_document_asks_for_nothing_knowable_not_for_nothing`.
    `test result: FAILED. 11 passed; 4 failed`. Reverted, 15 pass.
  - The mutation needs `#[allow(dead_code)]` on `passed_on` to compile at all
    under `warnings = "deny"`, or the run reports `error: method passed_on is
    never used` and says nothing about the tests.
- [x] The bundler's direct `Cartridge::read` call carries a comment saying whether a re-export is checked there and why.
  - Five lines above the call in `contracts()`: "`passed_on` is deliberately
    not run here. A re-export is checked once, where the cartridge enters the
    graph through `resolve`; re-checking it would make a contract listing fail
    on a neighbour's bad `export`, which is not this call's business."
- [x] `probe_nesting_is_not_discovered` still passes unchanged — this unit does not load a nested cartridge.
  - `test tests::manifest::probe_nesting_is_not_discovered ... ok`.
  - `src/tests/manifest.rs` is **untracked**, so `git diff` has no baseline for
    it and cannot be cited here — last pass it was, and it proved nothing. What
    can be shown instead: every change this file took is an append or an edit
    inside `an_unreadable_document_asks_for_nothing_knowable_not_for_nothing`,
    and none is inside this test; and the frontier the test marks is visible
    through the binary in `probe/end-to-end.sh`, which still prints
    `inner.store <- other` — `outer`'s need bound to an unrelated top-level
    cartridge because nothing loaded its own child. That is the same line pass
    two recorded.

## Verify and Proof

```sh
cd /Users/feb/dev/cartridge/.pearde/.lanes/the-manifest
cargo test --offline tests::manifest::
grep -n 'fn offered\|fn passed_on\|Cartridge::read' src/loader.rs
sh /Users/feb/dev/cartridge/.pearde/prds/the-manifest/probe/end-to-end.sh
```
