# the-ledger — retry pass two (implementation, box-run)

Verdict: DONE

The lane was already the pass's work, committed: `rebase session/s25699` ran
clean ("Current branch lane/the-ledger is up to date" — merge-base equals
`session/s25699`'s head, `d537b6d`). Every acceptance box across the three
specs was closed against output actually run this session. `cargo test
--offline` is **75 passed, 0 failed**; `cargo clippy --offline --all-targets`
is clean under `warnings = "deny"`; the probe is green end to end — 15 `ok:`
lines, `PROBE OK`, exit 0. The lane is one commit ahead of `session/s25699`
(`2dcce33 the-ledger: the tree, the walk, the override, the listing`),
working tree clean.

## What the last failure was, and what changed

The prior attempt's only red line was `spec02: exit 1`. I found two things the
standing tree got wrong for that block, and fixed both inside spec02's
footprint:

1. **The `disabled` comment in `Host::derived` opened "The answered fork:"** —
   spec02's third box says the comment states the answered rule and names *no
   fork*. Reworded to drop the fork name and state the rule only (the rule
   text itself was already right). Amended into the lane's standing commit;
   `grep -n 'fork' src/loader.rs src/tests/manifest.rs` now returns nothing.
2. **The verify block's `!`-negation line does not run on this board.** The
   greps it negates were already clean in the standing commit — I confirmed
   `git show b8641c7:src/loader.rs` and `:src/tests/manifest.rs` contain no
   `PROBE`, no "the only way a cartridge enters", no "is not discovered at
   all" — so a bash run of the line exits 0 (I ran it: 0). Yet spec02 exited 1
   while spec01, whose block has no `!` line, exited 0. The one line that
   cannot parse in nushell — this board's shell — is exactly that one
   (`! grep …` is not nushell). I replaced it in `specs/spec02.md` with an
   awk line of identical meaning (exit 0 iff no pattern in either file, exit 1
   otherwise — negative case tested against a planted pattern), which exits 0
   under both bash and nushell. The check's meaning is unchanged; only its
   spelling is portable now.

## Per-spec box status and verify output

**spec01 — the tree and the walk (4/4, all run this session).**
- Walk sub-proof re-run by hand: `outward()` cut to `vec![from]` ->
  `tests::ledger::a_walk_from_a_nested_scope_steps_outward` **FAILED**
  (`panicked at src/tests/ledger.rs:109`, "expected the outward binding, got
  []"); restored byte-identical (`git diff --stat src/ledger.rs` empty), test
  green again — `1 passed; 74 filtered out`.
- `cargo test --offline tests::ledger::` -> `test result: ok. 7 passed; 0
  failed; 0 ignored; 0 measured; 68 filtered out`. All seven names print from
  `src/tests/ledger.rs`.
- Whole suite: `cargo test --offline` -> `test result: ok. 75 passed; 0
  failed` (three binaries, last two `0 passed`). `grep -rn 'resolve(' src/ |
  grep -v tests/`: the only `Ledger::resolve` caller outside `ledger.rs` is
  `main.rs:292`, matching on `Bound`; the `loader.rs:508`/`lua.rs:217` hits
  are the unrelated path-resolution `resolve`.
- Clippy after `touch src/lib.rs`: `Finished 'dev' profile`, exit 0.

**spec02 — the profile is an override (3/3, all run this session).**
- `cargo test --offline tests::manifest::` -> `test result: ok. 15 passed; 0
  failed; 0 ignored; 0 measured; 60 filtered out` — the comment edits stand
  and no behaviour line moved.
- Scoped guard, run in isolation: `cargo test --offline
  tests::folders::watcher_reloads_a_folder_when_its_manifest_changes` ->
  `test result: ok. 1 passed; 74 filtered out` — the retain-before-extend is
  intact, so a profile naming a ledger cartridge by document path replaces
  the derived entry rather than duplicating it.
- The negated-grep line, run in its portable awk spelling under bash and under
  nushell: exit 0 both ways; negative case (pattern present) exits 1.
- `cargo test --offline
  tests::ledger::a_new_cartridge_is_available_and_not_started` -> `test
  result: ok. 1 passed`.

**spec03 — the listing names its problems (3/3, all run this session).**
- Probe through the binary (steps 7-9 as extended in
  `.pearde/prds/the-ledger/probe/the-ledger-is-the-tree.sh`):
  - step 7: `nobody.offers <- ?` printed, `run` exits 0;
  - step 8: empty root and nonexistent root both print nothing, exit 0;
  - step 9: `store.get <- ambiguous (far, other)` still named, `run` still
    exits non-zero.
  - Final: `PROBE OK`, exit 0, 15 `ok:` lines.
- Full suite after all of it: `test result: ok. 75 passed; 0 failed`.

Each spec's `## Verify and Proof` block was also run line-by-line as a script;
every line exits 0 (spec01: `s01a..s01d` all 0; spec02: `s02a..s02c` all 0;
spec03: `s03a..s03b` both 0).

## Findings

- **Correction to the earlier report's tooling note, kept standing.**
  `resources/` is absent at the repo root but the pearde install at
  `/Users/feb/dev/infra/pearde/resources/` has `knowledge.py` and
  `grammar.py`; `knowledge.py query "the-ledger registry subtree resolve"`
  returned strong hits, so no outside research was owed. The persona path
  `@references/personas/engineer.md` resolves there too.
- **`cargo fmt --check` drift** stands as prior passes left it: pre-existing,
  tab-formatted by hand, no `rustfmt.toml`, outside this footprint. The
  effective gate — `cargo test` plus clippy under `warnings = "deny"` — is
  clean.
- **Health floor:** nothing under the floor. Inside the specs' scope, exactly
  one line of comment changed (`src/loader.rs`, the fork-naming wording) and
  one spec verify line became shell-portable (`specs/spec02.md`); both are
  amendments to the standing commit, reported here. Nothing else moved; no
  defect outside scope observed.
- No contract word needed looking up; no `## Failure` written — the run is
  green.

## Corrections

Skeptic consult (`skeptic.md`, verdict CHANGE, 2026-09-12) — three corrections
judged real by the orchestrator, applied on the lane as one commit on top of
`2dcce33`. No acceptance box ticked; the boxes' texts amended where the shape
they describe changed.

### 1. Sibling visibility pinned, both sides

The ledger's reading holds: a nested cartridge's `provide` is visible to every
lookup from inside its parent's subtree — the walk passes the parent's scope —
and invisible to the graph outside the parent unless a parent re-exports it.
Reconciled with the-manifest's "satisfies its parent's needs and nothing else"
as naming the graph outside the parent; the-manifest's prd.md was not edited.

- `src/ledger.rs` — the private-key rule's doc (`Installed::provide`) and the
  `resolve` doc now state the reading; the module doc was checked and already
  agreed ("a key deeper than one level reaches it only where every parent
  between re-exported it").
- `.pearde/prds/the-ledger/specs/spec01.md` — the walk contract, as the walk
  implements it, stated after the `resolve` paragraph in "What already stands".
- New test `a_child_provide_is_seen_by_its_parent_subtree_and_not_by_the_graph_outside`
  in `src/tests/ledger.rs`: `outer/sib` binds `store.get` straight to
  `outer/inner`'s provide with no re-export anywhere (and the parent binds it
  too), while a top-level `stranger` and a lookup at the root itself read `?` —
  the key never left the parent's subtree. Both asks are declared needs, pinned
  through `bindings()` as well. On the live binary: `store.get <- outer/inner`
  printed, `exit=0`.

### 2. The ledger's read now agrees with the host's

- `src/loader.rs` — `Cartridge::passed_on` made `pub(crate)` (doc note added);
  `src/ledger.rs` `read_entry` runs it after `Cartridge::document`, so an
  entry whose document fails the re-export check carries `unread` with the
  host's reason and `zirkle ledger` exits 1 on a tree the host refuses to
  load. Verified live on the skeptic's fixture: `ledger` and `list` now print
  the same refusal line for `outer` and both exit 1. The doc claims at
  `ledger.rs` (`offers`, module doc, `read_entry`) now describe the code.
- `src/tests/ledger.rs` — `a_walk_from_a_nested_scope_steps_outward` rebuilt on
  a tree every valid document passes: `deep` declares the need
  (`needs: ["outer.top"]`), `outer/inner` is silent, `outer` provides
  `outer.top`; the walk still crosses three scopes (cut to the asker's own
  subtree the need reads `?`), the ask is a declared need pinned through
  `bindings()`, and the test asserts every document on the tree reads. The
  asker-exclusion the old test's doc claimed to pin is **kept as a synthetic
  resolve() unit test** — it already lives in
  `a_walk_answers_from_the_asker_subtree_then_steps_outward`
  (`resolve("other", "store.get")` → `None` while `other` provides the key),
  and the rebuilt test's doc comment says so; no separate new test was added.
  New test `a_dangling_re_export_is_unread_on_the_ledger_read_too` pins the
  read agreement itself (host's reason carried verbatim, declarations empty,
  the child below still reads).
- `spec01.md` — box 1's narrative amended to the shape actually proven
  (dangling-re-export shape named as refused-by-format, asker-exclusion named
  as synthetic); the "on the same terms `CartridgeInfo` keeps them apart"
  sentence replaced with what the read now actually does.
- `probe/the-ledger-is-the-tree.sh` — existing steps re-checked: the new read
  changes no expected line or exit on any existing fixture (all pass
  `passed_on`), so no fixture moved. A step 6b was added to pin the new
  behaviour end to end: a dangling re-export is refused on the ledger listing
  with the host's reason and a non-zero exit. (This edit lives in the main
  checkout's copy, the one this lane's gate runs; the lane commit predates the
  landing's rewrite of the prd files.)

### 3. `Ledger::root()` deleted

`grep -rn "\.root()" src/` confirmed no caller. `pub fn root()` and the
`root: PathBuf` field it read are gone from `src/ledger.rs`; `scan` no longer
stores it. No Cargo.toml dependency touched.

### Gates (from the lane, after all three corrections)

- `cargo test --offline` — `test result: ok. 77 passed; 0 failed; 0 ignored; 0
  measured; 0 filtered out` (was 75; +1 sibling-visibility test, +1 dangling
  re-export test; the rebuilt nested-walk test keeps its name).
- `cargo clippy --offline --all-targets` — clean, no warnings.
- `bash /Users/feb/dev/cartridge/.pearde/prds/the-ledger/probe/
  the-ledger-is-the-tree.sh` — green end to end, `PROBE OK` (now 17 `ok:` lines,
  including new step 6b).

Files touched: `src/ledger.rs`, `src/loader.rs`, `src/tests/ledger.rs` (lane
commit); `.pearde/prds/the-ledger/specs/spec01.md`,
`.pearde/prds/the-ledger/report.md`,
`.pearde/prds/the-ledger/probe/the-ledger-is-the-tree.sh` (main checkout; prd
files are outside the lane tree).
