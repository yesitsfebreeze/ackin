---
complexity: 5
footprint:
  - src/main.rs
  - src/loader.rs
  - src/tests/manifest.rs
---

# spec03 — what a cartridge asked for is listed beside what it provides

The capability request is only one document if there is one place to read it.
`zirkle list` already prints what each cartridge provides and what it needs;
this unit puts the re-exports and the four grant lists on the same line, so
there is no second view and no separate policy readout to keep in step.

## What already stands

`CartridgeInfo` carries `export: Vec<String>` and `grant: Grant` beside its
existing `inject` and `provide`. `Host::manifest` fills them from
`loader::resolve`, which reads the document as data — so a **disabled**
cartridge still reports what it asked for, because a disabled cartridge has
still declared it.

`list()` in `src/main.rs` appends `exports`, then `reads`, `writes`, `net` and
`execs`, each omitted when empty. Run on a nested fixture it prints:

```
outer  outer  exports inner.store  writes cache  execs rg
  inner.store <- other
other  other  provides inner.store
```

`probe/end-to-end.sh` builds that fixture in a directory made at run time and
is the check.

## What is left

*(Closed. Kept for the record of what the unit was asked to fix.)*

Two things the printed line got wrong, both visible in the output above.

A blank grant path printed as blank. `zirkle list` on a document declaring
`"grant": {"read": ["   "]}` printed `p  p  reads    ` — spec01 closed the
validation hole, and this unit's box is that the listing can no longer print a
grant with nothing in it.

`Host::manifest` swallowed the resolve error: `Err(_) => (Vec::new(),
Grant::default())`. A document whose re-export is bad therefore read as
*declaring nothing* in the `export` and `grant` columns while the `error`
column carried the real message. Two readings of one document on one line.
The grant columns are absent rather than empty when the document could not be
read, so an unreadable document never looks like a document asking for nothing
— which is the tightest policy and must stay meaningful.

## Acceptance

- [x] `zirkle list` on a cartridge declaring a grant with a blank path exits non-zero with the same message spec01 produces, rather than printing an empty `reads` column.
  - `probe/the-listing-reads-the-document-once.sh` section 1:
    ``blank  blank  error: …/blank/cartridge.json: `grant.read` entry `   ` must be a nonempty exact path`` then `exit 1`.
  - The section fails outright if the message is absent, if `reads` appears at
    all, or if the exit is 0.
- [x] `zirkle list` on a cartridge whose document fails to read prints the error and prints no `exports` or grant column for it.
  - Section 2 lists the **same folder twice**, once enabled and once disabled,
    and requires the message from both:
    ```
    bad  bad  error: …/bad/cartridge.json: `nobody.provides.this` is passed on, but nothing inside this cartridge offers it
    off  bad  (disabled)  error: …/bad/cartridge.json: `nobody.provides.this` is passed on, but nothing inside this cartridge offers it
    exit 1
    ```
    The section asserts the message appears exactly twice, that the disabled
    line carries one, that neither line prints `exports` or a grant column, and
    that the exit is non-zero.
  - This box was ticked last pass on the enabled half only. A disabled entry
    then printed `bad  bad  (disabled)` and exited 1 with no message anywhere,
    which is a worse readout than the one this unit set out to fix.
    `CartridgeInfo` now carries `unread` beside `error` — the document's own
    failure and the entry's evaluation failure are two different facts, and
    `Host::manifest` fills `unread` for disabled and enabled entries alike
    while suppressing `error` when it would repeat it. Keeping them apart is
    also what leaves `tests::folders::malformed_manifests_fail_without_evaluating_entries`
    passing untouched: its `error.is_none()` is about evaluation, and it still
    holds.
  - `an_unreadable_document_asks_for_nothing_knowable_not_for_nothing` asserts
    the same three things through `Host::manifest` for both entries.
- [x] `zirkle list` on a document declaring `provide`, `export` and all four grant lists prints each one exactly once, in one line per cartridge.
  - Section 4, on an **enabled** fixture, because `provides` is filled from the
    evaluated component and is the one column a disabled entry cannot show:
    ```
    on  on  provides on.key  exports in.key  reads data  writes cache  net api.example.com  execs rg
    ```
    The section asserts the output is exactly one line and that each of the six
    columns occurs exactly once, counted with `grep -o`.
  - Last pass this box was ticked on the disabled fixture below, whose line has
    no `provides` column at all — evidence that could not establish the first
    of the three declarations the box names.
- [x] A disabled cartridge still prints its `exports` and grant columns.
  - Section 5, the same folder as section 4 with the entry disabled:
    `off  on  (disabled)  exports in.key  reads data  writes cache  net api.example.com  execs rg`,
    each column once, `exit 0`.
  - `a_disabled_cartridge_still_declares_what_it_asked_for` asserts it through
    `Host::manifest`. The document is data, so it is readable whether or not
    the entry is evaluated.
- [x] `probe/end-to-end.sh` runs from a clean checkout of the lane and its output matches what this spec quotes, modulo the temporary directory.
  - The probe prints five lines in two sections. The three this spec quotes are
    the three under `--- list ---`, and they match exactly:
    ```
    outer  outer  exports inner.store  writes cache  execs rg
      inner.store <- other
    other  other  provides inner.store
    ```
    The other two are the section's own heading and the second section's single
    refused document. `exit 0`.
  - `probe/old-documents-still-load.sh` also passes, and **no longer stubs a
    `ui` file**: `--- 15 documents copied; list exits 0 only if every document
    reads ---`, fifteen lines, `exit 0`. Its one printed error is
    `ui  ui  error: …/ui/ui/index.ts: No such file or directory`, which is now
    what it always was — a fact about the fixture's tree, not a document that
    would not read. Last pass the stub was added to work around `list` exiting
    non-zero on it; the cause is fixed instead, in `loader::document`.

## Verify and Proof

```sh
cd /Users/feb/dev/cartridge/.pearde/.lanes/the-manifest
cargo build --offline --bin zirkle
sh /Users/feb/dev/cartridge/.pearde/prds/the-manifest/probe/end-to-end.sh
sh /Users/feb/dev/cartridge/.pearde/prds/the-manifest/probe/old-documents-still-load.sh
grep -n 'fn list' -A 30 src/main.rs
```
