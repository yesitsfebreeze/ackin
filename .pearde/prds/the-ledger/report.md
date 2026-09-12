# the-ledger — pass one

Verdict: QUESTION

The build went through. `cargo test` is **68 passed, 0 failed** with the ledger
standing, `warnings = "deny"` clean on `--all-targets`, and the probe green. It
went through by my picking two forks the contract does not settle, and both
picks are one line each. That is what the questions are.

Workflow followed: `probe-then-spec`, run to its question. No second file is
being drafted for it — its own `## Use when` names this run ("the first pass of
this same route, which stops at the question instead of continuing past it"), so
the recurring job already has a file. That is reported here rather than written.

Record queried first: 18 hits, 17 strong, no gap enqueued. `[[260912-466e]]`
carried the two open contracts this PRD inherits and was the whole research.
What this build learned went back as `[[260912-2afa]]`.

## What stands, uncommitted, in the lane

`/Users/feb/dev/cartridge/.pearde/.lanes/the-ledger`

- `src/ledger.rs` — new. `Ledger::scan(root)` derives every installed cartridge
  off the filesystem; an entry is keyed by its `/`-joined path from the root, not
  by its name. `Ledger::resolve(from, key)` is the outward walk: the asker's own
  children first, then its parent's, out to the root.
- `src/loader.rs` — `Host::derived` makes one entry per installed cartridge
  before `init.lua` is read at all; `Host::entries` lays the profile over it as
  an override. The profile is no longer the manifest of record.
- `src/main.rs` — `zirkle ledger` prints the tree with every need bound, and
  exits non-zero when a document would not read.
- `src/lib.rs` — one line.
- `.pearde/prds/the-ledger/probe/the-ledger-is-the-tree.sh` — builds its fixture
  tree in a temp directory at run time, and proves six things.

The exact symptom `[[260912-466e]]` recorded is gone. `outer` needing a key its
own child provides, with an unrelated top-level cartridge providing the same key,
now prints `store.get <- outer/inner` instead of binding to the stranger.

## The two questions, and what the build was doing at each

**Q1 — what a newly installed cartridge does.** Hit while wiring `Host::entries`
to the ledger. Auto-registration makes a cartridge an entry the moment its folder
appears; whether that entry *runs* is a separate fact the contract never states.
One line decides it — the `disabled` default in `Host::derived` — and exactly one
assertion separates the answers. `src/tests/bridge.rs` rewrites `init.lua`
without a folder and asserts the cartridge stops; under a ledger it does not,
because the list is no longer the record. `disabled: true` is 68/68;
`disabled: false` is 67/68 with that one assertion down. The PRD's own words
("neither involves editing a list") point at `false`. [[the-resolver]]'s words
("uninstalling is the absence of a launch", nothing runs until a tool is named)
point at `true`. Two board contracts pointing opposite ways is not a fact a build
can find. The tree is left at `true`, green, with the line commented as the fork.

**Q2 — two cartridges of one scope offering one key.** Hit at step 5b of the
probe. The settled rule — two cartridges may provide the same key without
colliding — holds across subtrees and does not hold inside one. With `other`
providing `store.get` at the top and `far` re-exporting the same key from
`far/deep`, a root-level ask prints `store.get <- far`: first in path order,
silently, no diagnostic, the other candidate never reached. Sorting by path makes
it deterministic across runs; it does not make it chosen.

## Findings

**A non-cartridge folder does not extend a subtree, and that hides things.**
`vendor/hidden` holds a document; `vendor` does not. `vendor/hidden` is therefore
in no subtree and reachable by nobody. This falls straight out of "a subtree is a
chain of cartridges" and needed no decision, but it means grouping installed
cartridges into plain folders under the root silently uninstalls them. Not a
fork — a consequence worth knowing before someone tidies a directory.

**Two claims in the tree that this build makes false.** `src/tests/manifest.rs`
carries `probe_nesting_is_not_discovered` ("a nested cartridge is not discovered
at all. The profile is still the only way a cartridge enters the graph") and
`a_parent_passes_an_inner_key_outward_by_naming_it` ("the inner cartridge is not
an entry of the profile and is not visible as one"). Both are true of the tree as
committed and both are what this PRD exists to delete. They pass today only
because `Host::derived` filters to `parent().is_none()` — a ledger that derived
nested entries too flips both. `src/tests/manifest.rs` is [[the-manifest]]'s
footprint, not mine; whoever specs this PRD's implementation must budget for
editing it, and it is named here rather than edited.

**A check that could not fail, found and left alone.** `src/main.rs`'s
`ledger_lines` counts unread documents to set the exit code, the same shape
`list` uses. Nothing here is weaker than that; noted only because the pairing was
checked rather than assumed.

**Footprint overlap across the board, for scheduling.** This PRD writes
`src/loader.rs` and `src/main.rs`. [[the-resolver]] names both as the things it
changes ("`deps()` in `src/main.rs`", "the inject-to-provider search in
`src/loader.rs`"). They cannot run at once on one lane.

## Three things that cost a test run each

Recorded in full as `[[260912-2afa]]`; the short form, because the next worker
will otherwise pay for them again.

1. Match an override by the file it resolves to, never by the string written. A
   profile may name the folder or the document inside it; `Entry::file` already
   makes those one cartridge, and comparing raw paths makes them two.
2. An override **replaces** the derived entry, it does not merge into it. Two
   profile entries may name one folder, and any positional overwrite loop eats
   the second. Collect the named files, `retain` the rest, extend. Instancing
   survives untouched.
3. Deriving nested cartridges as top-level entries puts an inner `provide` into
   the flat runtime registry, which is the settled namespacing rule inverted —
   and the compiler cannot see it. Two tests can.
