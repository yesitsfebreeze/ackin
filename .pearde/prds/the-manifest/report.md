# the-manifest — the document is also the capability request

Verdict: DONE

Pass three of `probe-then-spec`, the implementing pass, and its correction run
after the skeptic. `specs/` was already populated by pass two and the contract
had not changed, so no new unit was written: the twenty-two boxes were closed
instead, each with the command that closed it and its output written under it.
That is step 8's first branch, taken deliberately.

**Gate, run in `/Users/feb/dev/cartridge/.pearde/.lanes/the-manifest`:**
`cargo test --offline` — **68 passed, 0 failed** (63 at the start of this pass,
plus five). `tests::manifest::` names **15**, all from `src/tests/manifest.rs`.
`cargo clippy --offline --all-targets` clean after `touch src/lib.rs`, with the
`Checking zirkle` line so it is a real compile. `cargo doc --offline --no-deps`
clean. All three probes exit 0.

**Boxes: 22 of 22 ticked, 0 open**, and none reading false — spec01 6/6,
spec02 5/5, spec03 5/5, spec04 6/6.

The tree is uncommitted and stays that way.

## The five corrections, and a fourth round of record fixes

**1. spec01 box 1 read false on the board.** Fixed at the source: the
replacement text is now in `spec01.md` itself, not only in this report. The box
reads "…the count is at least the ten pass two left plus one for each box below
that requires a new test", and its evidence is `15 passed` with the five new
tests named and each tied to the box that required it.

**2. spec03 box 3 was ticked on evidence that could not establish it.** The
quoted line was a *disabled* fixture, which has no `provides` column at all
because `Host::manifest` does not evaluate a disabled entry — so the first of
the three declarations the box names was the one thing the line could not show.
The probe now has a fifth section with an **enabled** fixture, and the box
quotes it:
`on  on  provides on.key  exports in.key  reads data  writes cache  net api.example.com  execs rg`
— one line, each of the six columns counted exactly once with `grep -o`. The
disabled fixture stays as its own section, for box 4, which is what it can
prove.

**3. spec03 box 2 was true only for enabled entries — now fixed, and nothing
was bent.** A disabled entry with a bad document printed `bad  bad  (disabled)`
and exited 1 with no message anywhere. My previous report said one field could
not carry it without breaking
`tests::folders::malformed_manifests_fail_without_evaluating_entries`, outside
every footprint. That was true of one field and false of the design.
`CartridgeInfo` now carries **`unread` beside `error`**: why the *document*
would not read, against what evaluating the *entry* hit. Different facts, known
at different times — `unread` is known whether or not anything tried to load
the cartridge, which is the premise of the document being data. `Host::manifest`
fills `unread` for disabled and enabled alike and suppresses `error` when it
would only repeat it. The folders test's `error.is_none()` is about evaluation
and still passes, untouched. Recorded as [[260912-6a8c]], which supersedes the
ruled-out half of [[260912-8bef]].

**4. `list()`'s doc comment was false and the fixture had been changed to fit
the code.** `grant.is_none()` meant "any part of `resolve` failed", not "the
document would not read" — so a manifest declaring `"ui":"ui/index.ts"` whose
file is merely absent parses perfectly and made `zirkle list` exit 1. That is
why the probe had started stubbing a `ui` file. Fixed at the cause, not the
comment: `Cartridge::read` is split into `Cartridge::document` — the document
checked against itself and nothing else — and the file resolution that follows
it. `loader::document` wraps the first with `passed_on`, and the listing uses
it. `list()` now counts `unread.is_some()`, which is what its comment says.
`probe/old-documents-still-load.sh` **no longer stubs anything**: fifteen
documents read, exit 0, and the missing `ui` file prints as the fact about the
tree that it is. Pinned by
`a_missing_file_the_document_names_does_not_blank_its_declarations`.

**5. Four loose quotes corrected**, each against the file as it now stands:
- spec01 box 3 — the grep returns **16 lines across three structs**, not 5. The
  box now says so and names which four are `Cartridge`'s (166/170/174/179,
  under `deny_unknown_fields` at 147), and says plainly that reading the grep
  alone does not establish the box.
- spec04 box 1 — the substring loop proved nothing (`ui` matches "required").
  Replaced in the box by a grep for each field **as a quoted JSON key**:
  all fifteen return exactly 1. The spec's own `## Verify and Proof` loop is
  left as written; replacement under `### Edits`.
- spec04 box 5 — `^-.*///` returns **two** lines, both `Cartridge::read`'s
  method header, rewritten when it was split rather than deleted. No field's
  doc comment is touched: thirteen on `Cartridge` and `Grant`, with `name` and
  `entry` carrying none and never having. The file holds **119** `///` lines,
  82 indented — not the 68 quoted last pass.
- spec03 box 5 — `end-to-end.sh` prints **five** lines in two sections; the box
  now names the three under `--- list ---` as the ones that match.

**A fourth correction, this round.** spec01 box 4's evidence said "two callers,
both binding the whole struct", naming `Host::component_of` as the second.
`component_of` does not call `resolve` — it reaches it through `load_entry` ->
`load_component`, which *is* `src/lua.rs:217`. There is exactly **one** direct
call site in `src/`. The box's claim — that no caller destructures a tuple —
holds either way; the sentence under it did not, and is corrected in
`spec01.md`.

One more of my own, found while correcting box 5 of spec02: that box cited
`git diff src/tests/manifest.rs` as showing `probe_nesting_is_not_discovered`
untouched. **The file is untracked, so `git diff` has no baseline and proved
nothing.** The box now says that, and proves the frontier unmoved through the
binary instead — `end-to-end.sh` still prints `inner.store <- other`.

## What this pass changed

`src/loader.rs` — a sixty-line `//!` header that is now the document's only
written definition. `Grant::check` agrees with the key check on what blank
means. The one-level walk left `offered` for `fn nested`, whose doc comment is
the layout rule; the file name is `pub const MANIFEST`. `contracts()` says why
it does not re-check a re-export. `Cartridge::read` split into
`Cartridge::document` plus file resolution, with a free `loader::document`
beside `resolve`. `CartridgeInfo.grant` is `Option<Grant>` and `unread` is new.
`Declared` lost `export` and `grant`, which now have one reader each.

`src/main.rs` — `list()` prints grant columns only for a document it could
read, prints `unread` or `error` (never both, they are never both set), and
answers how many **documents** could not be read; `Command::List` exits 1 on
any.

`src/tests/manifest.rs` — five tests, 10 → 15.

`probe/the-listing-reads-the-document-once.sh` — five sections, the proof of
spec03's first four boxes through the real binary.
`probe/old-documents-still-load.sh` — the `ui` stub removed.

## What was falsified rather than asserted

Three boxes claim a check can fail. Each was made to fail against the tree as
it now stands, then reverted:

| mutation | result |
|---|---|
| `Grant::check` back to `p.is_empty()` | `a_blank_grant_path_is_refused_like_a_blank_key` — 14 passed, 1 failed |
| `nested()` made recursive | that test **and** `a_grandchild_key_reaches_the_top_only_through_every_level` — 13 passed, 2 failed |
| `passed_on`'s call removed from both readers | 11 passed, **4 failed** |

The third needs `#[allow(dead_code)]` to compile at all under `warnings =
"deny"`; without it the run reports `error: method passed_on is never used` and
tells you nothing. Recorded as [[260912-f295]].

## Findings

**The record's query scoring is unusable on this contract**, as pass two
reported: the PRD's own question returns unrelated hits with high scores.
Querying a narrower phrase got three strong hits that were all real —
[[260912-466e]], [[260912-1501]], [[260912-854a]]. A workaround, not a fix.
Defect in the scorer, outside this board.

**`Cartridge::check` now runs before the entry and `ui` are located**, where it
used to run after. A document with both a declaration error and a missing file
now reports the declaration error. That is the better order — the document's
own faults first — but it changes which message some manifests print, and no
box asked for it. Named here because it is a behaviour change that fell out of
the split. Recorded in [[260912-6a8c]].

**The test whose `error.is_none()` I twice explained was never able to fail.**
`git show HEAD:src/loader.rs` shows `Host::manifest` hardcoding `error: None`
in the `if entry.disabled` branch, at HEAD, before this PRD started. So
`tests::folders::malformed_manifests_fail_without_evaluating_entries`'s
`assert!(host.manifest().unwrap()[0].error.is_none())` could not fail for a
disabled entry under any change to the document's reading — it is structurally
unfalsifiable, not, as pass three's report called it and this pass repeated, "a
proxy for 'not evaluated'". What it still has teeth for is only that
`manifest()` returns `Ok` with at least one entry; the guarantee its name
claims is carried by `assert!(host.fiber_of("disabled").is_none())` two lines
below. This is the one thing this pass believed about the tree and did not
check — inherited from my own previous report, which is exactly how a wrong
census survives a pass. The `unread` split is right for its own reasons, and
none of them are this test. A check that cannot fail, in a file outside every
footprint: reported, not touched. Recorded as [[260912-777a]], together with
the exit-code leak below, because the wrong sentence outlived two passes.

**`a_missing_file_the_document_names_does_not_blank_its_declarations` promises
more than it asserts.** `grant` and `export` survive a missing `ui` file, and
those are what the test asserts. `provide` does not: it is filled from the
evaluated component, so the line reads
`u  u  reads data  error: .../u/ui/index.ts: No such file or directory` with no
`provides` column on it at all. The assertions are all true and no box depends
on the difference, but the name overreaches — "its declarations" is three
fields and the test covers two. Named here rather than renamed, because
renaming is a code change this round does not want.

**A profile entry naming a folder that is not there now exits 1, where at HEAD
it exited 0.** `gone  gone  error: .../gone/cartridge.json: No such file or
directory`, and the run is non-zero. At HEAD `list()` returned `()` and
`Command::List` had no per-entry exit at all. By `loader::document`'s own new
sentence — an `Err` from it "never means the tree around it is incomplete" — an
absent folder *is* the tree being incomplete, and yet it lands in `unread`,
because there is no document to fail to read and no document to read. This is
the one place the new distinction leaks. It is defensible: a profile naming a
folder that does not exist is a broken profile, and a listing that reports
success on one is worse. But no box covers it and no test pins it, so it is on
the record here rather than left for [[the-ledger]] to find. Also in [[260912-777a]].

**The frontier is unmoved and deliberately so.** Nothing loads a nested
cartridge. `probe_nesting_is_not_discovered` passes and `end-to-end.sh` still
prints `inner.store <- other`. The subtree is a property of the document;
making it a property of the graph is [[the-ledger]] and [[the-resolver]].

## Health floor

The brief lists no file in the footprint under the floor, and none was found.
Nothing was refactored. The `read`/`document` split is a seam the contract
required, not a cleanup.

## Workflow probe-then-spec

| # | step | outcome |
|---|---|---|
| 1 | read-the-contract | done — the answered fork restated in one sentence; pass two's census re-checked by `grep`, not trusted: 15 manifests under `~/dev/sys/builtin`, 10 tests in `manifest.rs`, 2 callers of `resolve`. All three held. |
| 2 | query-the-record-first | done — queried before any outside research; three strong hits read, all used. [[260912-854a]] set aside as answering step 6's subject rather than this contract's. No outside research happened. |
| 3 | apply-the-answered-fork | already applied by pass two; re-checked against the tree. The fork named code, the split exists (`offered`/`passed_on`), no cut vocabulary survives. This pass pinned the layout the split assumed. |
| 4 | port-the-tests-the-cut-orphaned | not applicable — pass two cut no module, so no test was orphaned. Five tests were *added*, none ported. |
| 5 | attempt-the-build | done — every target: `cargo test --offline` 68 passed 0 failed, `cargo clippy --all-targets` clean, `cargo doc` clean, and the binary linked and run by three probes. Cache checked with `touch src/lib.rs`; the `Checking zirkle` line appeared. |
| 6 | separate-the-machine-failure-from-the-crate | **not applicable**, per the atomic's own step 0: the build printed its success line — `test result: ok. 68 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.16s`. The machine fault pass two carried is on record as cleared ([[260912-854a]]). Nothing to separate. |
| 7 | record-what-the-build-learned | done — [[260912-8bef]], [[260912-f295]], [[260912-6a8c]] and [[260912-777a]], all written from the board root. The third supersedes the ruled-out half of the first and the fourth retracts the reasoning under both, which is the expensive kind of correction to leave unwritten. |
| 8 | write-the-specs | done via step 0's branch — `specs/` was populated and the contract unchanged, so no unit was re-split. Twenty-two boxes closed, each with its command and output written in. |

No back-edge was taken.

### Edits

**spec04, the `## Verify and Proof` loop.** A check that cannot fail: it counts
substrings, so `ui` matches "required" and `net` matches "nonempty", and every
field would pass whether or not the header named it. Replace:

> ```sh
> for f in name entry binary ui selftest integration source provide needs export grant read write net exec; do
> 	printf '%s: ' "$f"
> 	sed -n '1,60p' src/loader.rs | grep -c "$f"
> done
> ```

with:

> ```sh
> for f in name entry binary ui selftest integration source provide needs export grant read write net exec; do
> 	printf '%s=%s ' "$f" "$(sed -n '1,60p' src/loader.rs | grep -c "\"$f\"")"
> done; echo
> ```
>
> Each field must be named as a quoted JSON key, which is how the header
> carries the document's shape. Anything less matches prose by accident.

The `read-the-contract` edit this pass reported has been applied to
`.pearde/workflows/read-the-contract.md` by the orchestrator. Nothing else is
owed against the route.

## Scores

complexity: 30
blast-radius: mid
workflow: probe-then-spec
