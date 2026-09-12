# Skeptic — the-ledger

Reviewed the landed work on main (`2dcce33`, `25ab31d`, record `0dd456a`) against
`prd.md`, `specs/spec01.md`, `specs/spec02.md`, `specs/spec03.md` and
`report.md`. Gates re-run on main: `cargo test --offline` — `75 passed;
0 failed`; `cargo clippy --offline --all-targets` — clean; the probe —
`PROBE OK`. Nothing below is a test failure; every finding is a box that does
not prove what it says, or a landing the standing direction forbids.

---

## 1. The outward walk grants a nested cartridge's private provide to its parent's other children — the sibling case is unpinned and contradicts the settled namespacing decision as written

**Claim.** The PRD (`.pearde/prds/the-ledger/prd.md:33-42`) declares the
namespacing decision in [[the-manifest]] *settled* and binding on this PRD's
data structure. That decision, verbatim
(`.pearde/prds/the-manifest/prd.md:38-40`): *"A cartridge's `provide` keys are
private to that cartridge's own subtree by default. A nested cartridge
satisfies its parent's needs and nothing else; the graph outside the parent
cannot see it and cannot bind to it."* The ledger's own doc repeats it
(`src/ledger.rs:45`: "Keys offered, private to this cartridge's own subtree";
`src/ledger.rs:163-168`: "A key one subtree over is invisible however
identical its name").

**Evidence.** The landed `resolve` makes a nested cartridge's `provide` a
candidate for *every* lookup whose walk passes its parent's scope — that
includes its parent's other children and all of their descendants. Built on
disk and run through the landed binary:

```
$ zirkle --dir <fixture> ledger
other  provides store.get
outer
outer/inner  provides store.get
outer/sib
  store.get <- outer/inner
exit=0
```

`outer` declares nothing; `outer/inner` provides `store.get`; `outer/sib`
needs it. `outer/inner`'s private key binds to `outer/sib`'s need with no
re-export anywhere — under the settled wording, `outer/inner` "satisfies its
parent's needs and nothing else", and `outer/sib`'s need is not its parent's.
(Under that reading the correct binding is `store.get <- other`, from the root
scope.) No box pins either behaviour: `grep -rn "sib"
src/tests/ledger.rs .pearde/prds/the-ledger/probe/the-ledger-is-the-tree.sh`
matches only two comments about other trees, and no test or probe step asks
from a sibling.

The pre-ledger runtime could not grant this visibility at all (a nested
cartridge is never a host entry, so the flat provider search could never bind
it), so this is newly granted by the ledger and rests on no pinned decision.

**Smallest fix.** One test in `src/tests/ledger.rs` pinning what a sibling may
see — whichever reading the board chooses — plus one line in the `ledger.rs`
doc (and `spec01`) saying which reading holds. If scope-visibility (a parent's
whole subtree sees its children's provides) is intended, the-manifest's
"satisfies its parent's needs and nothing else" needs a reconciling note,
because the code as landed breaks that sentence for `outer/sib`.

---

## 2. The nested-scope box is proven on a tree the document format refuses, by a lookup no valid document can make

**Claim.** `specs/spec01.md:58-63` (box 1): *"A lookup from a nested scope
steps outward through more than one scope, proven by a test … that fails
without the walk"* — with the narrative "`outer/inner/deep` asks for a key its
own subtree does not offer, `outer/inner` does not offer it, `outer`
re-exports it". `spec01.md:24-26` also claims an unreadable document is kept
from a readable one *"on the same terms `CartridgeInfo` keeps them apart"*, and
`src/ledger.rs:62-64` claims `export` *"is validated against the subtree at
document-read time, so a re-export here names something real."*

**Evidence.** The landed test's fixture (`src/tests/ledger.rs:89-105`) is
`outer` declaring `export: ["store.get"]` while its only direct child
`outer/inner` declares nothing. The format refuses exactly that shape
(`src/loader.rs:403-417`, `passed_on`), and it is refused on the path the
ledger does not walk: the ledger reads with `Cartridge::document`
(`src/ledger.rs:243-244`), which runs `check()` only, while the host's reader
(`src/loader.rs:495-506`) also runs `passed_on`. Same tree, two listings:

```
$ zirkle --dir <fixture> ledger
outer  exports store.get
outer/inner
outer/inner/deep  provides store.get
exit=0

$ zirkle --dir <fixture> list
outer  outer  (disabled)  error: …/outer/cartridge.json: `store.get` is passed on,
but nothing inside this cartridge offers it
exit=1
```

So `zirkle ledger` — "the registry the resolver reads" — exits 0 on a tree the
host itself refuses to load, and the doc claim at `ledger.rs:62-64` is false on
the ledger's own read path. Second, the format-forced deviation: a document
cannot declare `provide` and `needs` for the same key (`src/loader.rs:337-341`;
empirically: `{"provide":["store.get"],"needs":["store.get"]}` → "`store.get`
is provided here, so it is not also needed from outside", exit 1). The landed
test therefore calls `resolve("outer/inner/deep", "store.get")` directly for a
key `deep` itself offers — a lookup `bindings()` cannot make for any valid
document: a real need naming a key the asker provides is refused at read time,
and if the asker offered the key only via a valid `export`, its own child scope
would bind first. The asker-exclusion the test's doc comment claims to pin
("the case pass two never ran") is exercised only through this unreachable
shape. What does hold: the box's mutation claim — cutting the walk to one
scope fails both walk tests (`5 passed; 2 failed`), and removing
`e.path != from` fails the same two — the walk's mechanics are load-bearing.

**Smallest fix.** Rebuild the test on a tree every document would pass:
e.g. `deep` declares `needs: ["outer.key"]`, the intermediate scopes are
silent, and `outer` provides `outer.top` — the walk still crosses three scopes,
the ask is a declared need, and nothing rests on a dangling re-export. Then
amend the box text and the test's doc comment to the shape actually proven
(the asker-exclusion case is unreachable for valid documents and should be
pinned as a synthetic property or dropped). Separately, decide the ledger's
read: either run `passed_on` in `read_entry` so the ledger's `unread` and the
host's agree (and `zirkle ledger` exits 1 on the tree above), or delete the
false validation claim at `src/ledger.rs:62-64` and the "same terms" sentence
in `spec01`.

---

## 3. `Ledger::root()` landed with no caller

**Claim.** Standing user direction: this tree is a plain copy — remove
everything nothing uses; every new item must have a caller.

**Evidence.** `src/ledger.rs:115-117` adds `pub fn root(&self) -> &Path`.
`grep -rn "\.root()" src/` returns nothing — no production caller and no test
caller. The `root` field it reads (`src/ledger.rs:98, 110`) has no other
reader. The other new surface has callers (`len`/`is_empty`/`get`/`paths` in
`src/tests/ledger.rs`, `children` in `resolve`, `is_clashed` in `main.rs` and
tests); only `root()` is dead, and clippy does not flag it because the item is
`pub` in a lib crate.

**Smallest fix.** Delete `root()` and the `root` field, or point a test at it.

---

## Examined and dismissed — the profile entry naming a missing folder

**Lead.** A profile entry naming a missing folder now prints an error line and
exits 1 where the pre-ledger code exited 0; nothing pins either behaviour.

**Evidence.** Fixture: profile `init.lua` returning `{ { id = "ghost", path =
"ghost/cartridge.json" } }`, empty cartridge root. Landed main:

```
ghost  ghost/cartridge.json  error: …/cartridges/ghost/cartridge.json: No such
file or directory (os error 2)
exit=1
```

A build of `25ab31d^` (`d537b6d`, the pre-ledger baseline) on the same fixture
prints the identical line and exits 1 identically. The exit-1-on-unreadable
behaviour predates the ledger (`list` counts unread and `Command::List`'s `> 0`
exit was already there); the landed diff to `src/main.rs` adds only
`ledger_lines` and `Command::Ledger`. There is no exit change to pin and the
boxes are not silent about one — the lead dissolves.

---

## Verdict

CHANGE (the three corrections above)