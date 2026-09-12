---
complexity: 12
footprint:
  - src/loader.rs
  - src/lua.rs
  - src/tests/manifest.rs
---

# spec01 — the document declares what it provides, needs and asks the machine for

`cartridge.json` carried identity and entry only: `name`, `entry`, `binary`,
`ui`, `selftest`, `integration`, `source`. What a cartridge provided and what
it injected lived in the Lua entry, or in a process's `hello` output, so there
was no place to state a capability request without inventing a second file.
This unit puts all four declarations on the one document — `provide`, `needs`,
`export`, `grant` — and makes the document win over the entry wherever it
speaks.

## What already stands

`Cartridge` in `src/loader.rs` gained four fields, each `#[serde(default)]` so
the struct's `deny_unknown_fields` does not refuse a manifest written before
them. `Grant` is a struct of `read`, `write`, `net` and `exec`, and an absent
`grant` deserialises to `Grant::default()` — empty on all four, which is the
tightest policy and not the loosest.

`Cartridge::check` validates every declaration by reading the document alone:
a key must be a nonempty exact string with no `*` and no NUL; `provide` and
`export` may not repeat or overlap each other; `needs` may not name a key the
same document provides; `selftest` and `integration` must name a provided key
when the document declares any. `Grant::check` requires a relative path to stay
inside the cartridge folder, a `net` entry to be a host name or the bare `*`,
and an `exec` entry to be nonempty.

`loader::resolve` now returns a `Declared` struct — `entry`, `name`, `sources`,
`provide`, `needs` — in place of the three-element tuple. (Pass two also put
`export` and `grant` on it; the implementing pass moved those to
`loader::document`, whose whole job they are. See spec03.) `declarations()` in
`src/lua.rs` copies the document's `provide` into
`Component::provide` and its `needs` into `Component::inject` when either is
non-empty. A bare Lua path has no document, so it yields an empty `Declared`
and the entry stays the only source.

`src/tests/manifest.rs` covers it:
`the_document_declares_what_it_provides_and_needs`,
`the_capability_request_is_on_the_same_document`,
`an_empty_document_requests_nothing`,
`malformed_declarations_are_refused_without_evaluating_the_entry`, and
`the_document_overrides_what_the_entry_declares`.

## What is left

The checks below have not been run against this unit in isolation — the suite
was run whole. Run them, and close the two gaps they expose.

`Grant::check` tests `p.is_empty()` on a path but `k.trim().is_empty()` on a
key, so a `grant.read` entry of `"   "` is accepted where a `provide` entry of
`"   "` is refused. Make the two agree on `trim()`.

The `selftest`/`integration` cross-check is skipped whenever the document's
`provide` is empty. That guard is deliberate and must stay — `harness` in
`~/dev/sys/builtin` declares `selftest` with no document-level `provide` — but
it is currently unremarked. Say so in the code, so the next reader does not
read it as a check that forgot to fire.

## Acceptance

- [x] `cargo test --offline tests::manifest::` passes, every test it names comes from `src/tests/manifest.rs`, and the count is at least the ten pass two left plus one for each box below that requires a new test.
  - `test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 56 filtered out`,
    and `cargo test --offline tests::manifest:: | grep -c '^test tests::manifest::'`
    → `15`, so every name the run printed is from this file.
  - Ten from pass two, plus five: `a_blank_grant_path_is_refused_like_a_blank_key`
    (box two below), `a_cartridge_two_levels_down_is_not_offered` (spec02 box
    one), `a_disabled_cartridge_still_declares_what_it_asked_for` and
    `an_unreadable_document_asks_for_nothing_knowable_not_for_nothing`
    (spec03 boxes two and four), and
    `a_missing_file_the_document_names_does_not_blank_its_declarations`.
  - This box read "names ten tests" until this pass. Ten was impossible from the
    moment the spec was written, because box two below requires a test that did
    not exist. Replacement text and the reason are in the report under `### Edits`.
- [x] A `grant.read` entry of `"   "` is refused with the same wording a blank `provide` key is refused with, and a test in `src/tests/manifest.rs` fails without the fix.
  - `Grant::check` now tests `p.trim().is_empty()` and says
    ``` `grant.read` entry `   ` must be a nonempty exact path ```, against the
    key check's ``` `provide` entry `   ` must be a nonempty exact key ```.
  - Falsified: with `trim()` reverted to `is_empty()`,
    `test tests::manifest::a_blank_grant_path_is_refused_like_a_blank_key ... FAILED`
    — `test result: FAILED. 14 passed; 1 failed`. Reverted, 15 pass.
- [x] `Cartridge` still carries `#[serde(deny_unknown_fields)]`, and every one of `provide`, `needs`, `export`, `grant` carries `#[serde(default)]`.
  - `grep -n 'serde(deny_unknown_fields)\|serde(default)' src/loader.rs` returns
    **16 lines across three structs** — `Entry` at 74 with four defaults at
    78/80/82/84, `Cartridge` at 147 with four at 166/170/174/179, `Grant` at 186
    with four at 190/193/196/199 — plus one prose mention in the header at 32.
    The four this box names are `Cartridge`'s: 166 `provide`, 170 `needs`,
    174 `export`, 179 `grant`, each immediately above its field. Reading the
    grep alone does not tell you that; the file at those lines does.
- [x] `loader::resolve` returns `Declared` and no caller in `src/` destructures a tuple from it.
  - `pub(crate) fn resolve(path: &Path) -> mlua::Result<Declared>`.
  - **One** direct call site in `src/`, binding the whole struct:
    `src/lua.rs:217` `let declared = crate::loader::resolve(path)?;`, inside
    `Host::load_component`. `Host::component_of` reaches it through
    `load_entry` → `load_component` and does not call `resolve` itself; last
    pass this box's evidence counted it as a second call site, which was wrong
    about the tree. `grep -rn 'resolve(' src/loader.rs src/lua.rs src/main.rs`
    minus the definition returns that one line and nothing else.
  - `Host::manifest` no longer calls `resolve` at all — it calls the narrower
    `loader::document`, which is why `Declared` lost `export` and `grant` this
    pass. See spec03 box two.
- [x] The guard on the `selftest`/`integration` cross-check carries a comment naming the real manifest that requires it.
  - Four lines above `if !self.provide.is_empty()`: "`harness` in
    ~/dev/sys/builtin declares `selftest: "harness.selftest"` with no
    document-level `provide`, because its Lua entry is what provides."
- [x] `cargo clippy --offline --all-targets` is clean with `warnings = "deny"` in force.
  - After `touch src/lib.rs`: `Checking zirkle v0.1.0` then
    `Finished \`dev\` profile ... in 1.04s`, exit 0. The `Checking` line is
    there, so this is a real compile and not a cache hit.

## Verify and Proof

```sh
cd /Users/feb/dev/cartridge/.pearde/.lanes/the-manifest
cargo test --offline tests::manifest::
grep -n 'serde(deny_unknown_fields)\|serde(default)' src/loader.rs
grep -rn 'loader::resolve' src/ | grep -v '^src/tests/'
touch src/lib.rs && cargo clippy --offline --all-targets
```
