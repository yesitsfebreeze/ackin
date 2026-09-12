---
kind: routine
description: One legibility pass over the code — find the next thing that costs a reader more than the job needs, prove the cost with a count, and land the smallest change that lowers it.
uses:
  - usage: "[[run-usage]]"
    when: [making the tree easier to read, for a person or for an agent]
---

1. Read

- `memos/SYSTEM.md` — the frontier-mode law and the atomicity law are this
  routine's authority: no code without a caller, rename means rename, one
  claim per memo.
- a-reader-pays-per-unit-not-per-line — what this pass optimises and why
  a total is the wrong number. the-longest-unit-is-not-the-biggest-file
  is the dated baseline every probe is compared against.
- layers — the L0-L6 contract a split or a move must not break.
  dispatch — one core, many surfaces, the shape a merge lands in.
  one-memory — what a second copy of one thing costs a reader.
- method and the three sweep laws, which govern every count this routine
  takes: one-reading-is-not-a-measurement,
  read-twice-by-different-means, a-sweep-needs-a-negative-control.
- `git status --short` and `git log --oneline -10`. Code the tree is
  mid-change on is never this pass's candidate.
- lanes-not-a-shared-tree — every edit here is made in a lane:
  `just lane legible-<probe>` before the first edit, `just land legible-<probe>`
  after the gates. Never a direct edit of the trunk.

This routine is not [[quality]]. Quality asks *is there excess* — a second
copy, an abstraction with one user, a declaration living in Rust — and its
proof is the compiler: delete it and `just check` stays green. This one asks
*what does one reader pay*, and its proof is a count. A 233-line function has
no excess at all; every line is called, nothing is duplicated, and quality's
six probes cannot see it. A pass that finds excess hands it to [[quality]];
a pass that finds a false claim in the record hands it to [[improve]].

2. Probe

The caller may name one probe. With none named, walk in order and stop at the
first that yields a candidate.

**a. The long unit.** The largest thing a reader must hold whole.

```
for f in $(find src -name '*.rs' -not -path '*/tests/*'); do awk -v F=$f '/^[[:space:]]*(pub(\([a-z]+\))? )?(async )?fn /{n=$0;s=NR;d=0;o=1} o&&/{/{d+=gsub(/{/,"{")} o&&/}/{d-=gsub(/}/,"}"); if(d<=0&&NR>s){print NR-s, F":"s; o=0}}' $f; done | sort -rn | head -15
```

A long body is a lead, not a hit. A hit is a body doing more than one job —
the split is where the local variables stop being shared. A dispatch `match`
with one arm per verb is one job however long it runs; `src/rpc/src/server.rs`
is the standing example that is not a candidate.

**b. The two-subject file.** A file whose own `//!` header needs an "and".

```
find src -name '*.rs' | xargs wc -l | sort -rn | head -20
head -3 src/*/src/lib.rs
```

Every crate declares its job in its own `lib.rs`. A hit is a file, or a
crate, whose declared job is two jobs — the same split test SYSTEM
applies to a memo. Size is only where to look: line totals are a crate's
weight, never its reading cost (a-reader-pays-per-unit-not-per-line).

**c. The hop count.** One fact a reader must check in N places.

```
rg -n '\b16, *200\b' src
rg -oN '^\s*(pub )?const [A-Z_0-9]+' src --no-filename | sort | uniq -cd | sort -rn | head
```

the-index-tuning-pair-is-written-fourteen-times is the proving case: two
literals at fourteen sites, three of them separately reachable, so a partial
edit builds two index shapes and nothing goes red. A hit is one value whose
copies must agree and whose agreement nothing enforces
(a-copied-fact-has-no-dissent).

**d. The lying name.** Prose or a name that describes different code.

```
cargo test --test cited_paths
rg -n '^///' src -A1 | rg -B1 'fn [a-z_0-9]+' | head -40
```

two-doc-comments-document-the-wrong-function and
`a-citation-rots-quietly` are the two shapes: a true paragraph filed above
the wrong function, and a comment citing a layout the tree left behind.
`tests/cited_paths.rs` is the runner for the second; the first has none, so
it is read by hand. A name that lies costs a reader more than a long body,
because the body can be read and the name cannot be checked.

**e. The cold open.** What must be loaded to change one thing.

Take the file a real recent commit touched (`git log --oneline -10`), and
count what a cold agent must open before it may edit: the crate's `lib.rs`,
each type it names, each caller it must not break. Six or fewer is the job;
more than that is the candidate, and the fix is almost always (b) or (c),
never a new abstraction. dispatch is what a good answer looks like.

3. Pick

One candidate per pass. One line: which probe found it, the count that makes
it a candidate, and what lowering it costs. Prefer, in order: a name that
lies, a fact written N times, a unit doing two jobs, a file holding two
subjects, a cold open over six.

4. Prove

A split with no number is a preference. Before editing, take the count twice
by two different instruments (read-twice-by-different-means) and give the
sweep one known-good subject it must not flag (a-sweep-needs-a-negative-control).
Quote the before number. The after number comes from the same instrument in
step 5, or the pass did not land.

5. Land

- The smallest diff that lowers the count. Extracting a named function from a
  long body is the whole change; no trait, no builder, no module reshuffle
  rides along. Deletion beats extraction where a caller can hold the lines.
- `just check` green with the layer headers unedited, or the move broke
  layers.
- Non-trivial logic the change moves keeps one runnable check, and a rule
  that must hold from now on is a Rust test under `tests/`, never a recipe or
  a hook (gates-are-tests).
- `just all` green, then `just land legible-<probe>`. Nothing lands red.

Bigger than one pass — it becomes one `kind: work` memo, `level: 10`, in
`memos/intake/`, `Do` naming the files and `Check` naming the command that
re-takes the count. [[work]] runs it later.

6. Record and loop

- A verdict worth keeping is one memo: `knowledge` for a count and what it
  measures, `decision` for a candidate deliberately refused and why,
  `insight` for a connection across probes. One claim per memo in the same
  change. Refusing a candidate is a result — record it so the
  next pass does not re-argue it.
- Author through `memo` writes; report results and warnings. One line: probe, candidate, count before and
  after.
- Back to step 2 with the next probe. The loop ends when a full walk of the
  five yields nothing, or when the caller stops it.
