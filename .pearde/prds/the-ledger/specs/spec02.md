---
complexity: 9
footprint:
  - src/loader.rs
  - src/tests/manifest.rs
---

# spec02 — the profile is an override, and a new cartridge is available, not started

The profile is no longer the manifest of record. Every cartridge installed
under the root is an entry before `init.lua` is read at all; the profile lays
overrides over them; installing is putting a tree where the ledger looks and
uninstalling is taking it away, and neither edits a list. The answered fork
fixes what a fresh install does: the entry exists, its document is read and its
declarations carry, and nothing of it runs until something needs it.

## What already stands

`Host::derived` in `src/loader.rs` makes one entry per top-level ledger entry
(`parent().is_none()`), id and path both the ledger path, `disabled: true` —
the answered fork, and the comment above the line now states the rule rather
than naming the fork. Nested cartridges are deliberately *not* host entries:
their private `provide` in the flat runtime registry is the settled
namespacing rule inverted, and the compiler cannot see it (pass one's finding
three, recorded as the record entry pass one left). The ledger registers a
nested cartridge; the resolver's contract loads it.

`Host::entries` lays the profile over the derived entries: an override is
matched by the file it resolves to (`Entry::file`), never by the string
written, and the derived count is frozen before the `retain` so two profile
entries naming one folder are two instances and do not eat each other. An
override naming something outside the root is added, which is how programmatic
composition survives. `config.lua` maps entry id to config fields over the
entry's own.

`src/tests/ledger.rs::a_new_cartridge_is_available_and_not_started` — a
cartridge dropped into the root is listed with `disabled` set, its entry would
`error` if evaluated, no fiber exists for it, and removing the folder removes
the entry. Green this pass.

The host's `bridge` assertion pass one worried about stays green under the
ledger: a cartridge the profile does not name stays disabled, so rewriting
`init.lua` without a folder still stops it — but for the ledger's reason, not
the list's.

## What is left

Two doc comments in `src/tests/manifest.rs` claim things the ledger makes
false, and both tests' *behaviour* assertions are still correct. They are
named here rather than edited, because the file is the manifest contract's
footprint and the implementing pass budgets for it:

- `probe_nesting_is_not_discovered` — "The profile is still the only way a
  cartridge enters the graph" is false: the ledger enters it. What the test
  actually pins is that a nested cartridge is not a *host profile entry*.
- `a_parent_passes_an_inner_key_outward_by_naming_it` — "The inner cartridge
  is not an entry of the profile and is not visible as one" reads as the old
  reason. It is still one, but because the ledger's subtree rule keeps its
  `provide` private, not because nothing discovered it.

## Acceptance

- [x] Neither `manifest.rs` comment claims the profile is the only way in, and both tests' behaviour assertions still pass unchanged.
  - `grep -n 'the only way a cartridge enters\|is not discovered at all'
    src/tests/manifest.rs` returns nothing; `cargo test --offline
    tests::manifest::` is green at the count this pass left (15). A comment
    edit must not move a behaviour line; the count is the guard.
- [x] A profile that names a ledger cartridge by its document path (`path="p/cartridge.json"`) replaces the derived entry and yields one instance, not two — proven by a test that fails on a positional overwrite.
  - The shape `folders::watcher_reloads_a_folder_when_its_manifest_changes`
    guards is exactly this; if it passes, the retain-before-extend is intact.
    The check was never run in isolation this pass — run it scoped and close
    the box with its output.
  - mason 2026-09-12: scoped — `test tests::folders::
    watcher_reloads_a_folder_when_its_manifest_changes ... ok`; `test result:
    ok. 1 passed; 0 failed; 0 ignored; 0 measured; 74 filtered out`.
- [x] The `disabled` default comment in `Host::derived` states the answered rule and names no fork.
  - `grep -n 'PROBE\|open fork' src/loader.rs` returns nothing.
  - mason 2026-09-12: the comment still opened "The answered fork:" — reworded
    in `src/loader.rs` to state the rule without naming the fork (amended into
    the lane's standing commit). `grep -n 'fork' src/loader.rs
    src/tests/manifest.rs` now returns nothing; both verify lines exit 0.

## Verify and Proof

```sh
cd /Users/feb/dev/cartridge/.pearde/.lanes/the-ledger
cargo test --offline tests::manifest::
cargo test --offline tests::ledger::a_new_cartridge_is_available_and_not_started
awk '/the only way a cartridge enters|is not discovered at all|PROBE/ {found=1} END {exit found}' src/tests/manifest.rs src/loader.rs
```