---
kind: routine
description: One quality pass over the code — find the next thing that is duplicated, over-built, or belongs in the record instead of in code, prove it with a number, and land the smallest change that removes it.
uses:
  - usage: "[[run-usage]]"
    when: [improving code quality, hunting a deletion, or asking whether the graph and the network still earn their cost]
---

1. Read

- `memos/SYSTEM.md` — the frontier-mode law is the whole authority for this
  routine: git is the archive, no code without a caller, rename means rename.
- method (how a claim is probed here), `layers` (the L0-L6 contract a
  unification must not break), `one-memory` (what a second copy of a thing
  costs), `the-test` (the twelve criteria a change fails the vision by
  breaking).
- `git status --short` and `git log --oneline -10`. Excess the tree is
  mid-change on is never this pass's candidate.
- lanes-not-a-shared-tree — the change this pass lands is made in a
  lane: `just lane quality-<probe>` before the first edit, `just land`
  after the gates, never a direct edit of the trunk.

This routine touches code; [[improve]] sweeps the record and touches none.
A pass that finds a false claim rather than excess code hands it to
[[improve]] and picks the next probe.

2. Probe

The caller may name one probe. With none named, walk in order and stop at the
first that yields a candidate — cheapest and most deletable first.

**a. Duplication.** Two copies of one thing.

```
rg -oN '^\s*(pub )?fn [a-z_0-9]+' src --no-filename | sed 's/.*fn //' | sort | uniq -cd | sort -rn | head -30
cargo machete
```

A repeated name is a lead, not a hit — `new`, `default` and `from` repeat by
trait. A hit is two bodies doing one job, or one list written twice. The
standing runners already hold three shapes of this and need no re-checking:
`tests/one_dispatch.rs` (a second dispatch table), `tests/layer_headers.rs`
(a crate importing across its layer), `tests/declared_dependencies.rs` (a
dependency nothing names) — and that last one matches any whole identifier,
so `cargo machete` is the second opinion that sees what it misses
(`an-identifier-satisfies-the-dep-gate`). `cargo machete` is the one tool
here the workspace does not carry: `cargo install cargo-machete` if the
command is missing.

**b. Over-building.** More structure than the job needs.

```
find src -name '*.rs' | xargs wc -l | sort -rn | head -20
```

A hit is an interface with one implementation, a config key for a value that
never changes, a builder for a struct with three fields, a trait no second
type implements. Size is only where to look: `src/rpc/src/server.rs` is long
because the dispatch table is one table on purpose.

**c. Unification.** Two systems that are one system.

```
head -3 src/*/src/lib.rs
```

Every crate declares its job and its layer in its own `lib.rs`. Two crates
whose declared jobs overlap are the candidate. `dispatch` is the shape a
unification lands in — one core, many surfaces — and `layers` is what a
merge may not violate. `store` already declares L4 and depends on an L6
crate (`store-depends-on-tick-loop`); that inversion is a known standing
candidate, not a new finding.

**d. Memo instead of code.** A declaration that is living in Rust.

```
rg -n 'const [A-Z_]+: *(&\[|\[)' src
```

`the-surface-is-declared-in-the-record` is the verdict this probe applies
and its boundary is the whole test: a **declaration** moves into the record
as a memo the watcher carries; **behaviour** stays code. The 23 operations
are behaviour. The three registries this probe was written against —
`DEFAULT_KINDS`, the focus and the routine tables — are declarations and have
already moved: kinds-are-declared, gravitons-are-declared and
routines-are-tools all read `status: done`, the record carries six
`kind: persona` memos, and the constant is not in the tree, so the probe no
longer returns them. What it
does return, checked 2026-09-08, is seventeen files of behaviour that stays:
`RETRY_DELAYS_MS` and the byte-unit array `U`, the hygiene pattern tables, the
retry and scale arrays. Anything else the probe returns is undecided and is
what this pass judges.

**e. Is the graph working.**

```
kern report
kern health
kern doctor
```

`doctor` is the one that names defects — dangling reasons, near-duplicate
twins, bare labels, a store older than the build. `health` is the shape:
kern count against the cap, gini, the tick queue against tick done, the
degraded counters, the embed stamp. Read every health number as
per store since 2026-09-06 (per-store-health-counters).
`graph-store` and `recall-pipeline` say what the numbers mean.
Behaviour, as opposed to counters, is answered by `just e2e`.

**f. Does the network need to be better.**

```
just eval-ground --path direct
just eval-ground --path distill
```

`src/gnn/` is the network, `tests/gnn_scale.rs` and `tests/e2e/gnn_recall.rs`
are its gates, and `measurement-verdicts` carries every GNN verdict already
taken. The law before any model change: reproduce the current number first.
Three of four benchmark instruments were deleted and the survivor reports a
different number than the record published
(`every-published-number-is-unreproducible`), and the GNN re-embeds the
same corpus differently in every process (open-work item 2) — so a
recall difference is unattributable until that closes. A model change
proposed without a reproduced baseline is refused by `positioning`'s claim
standard.

3. Pick

One candidate per pass. Say in one line which probe found it, what the excess
is, and what removing it costs. Prefer, in order: a second copy of one thing,
a declaration living in code, an abstraction with one user, a measured graph
defect, a model change.

4. Prove

A deletion with no number is a preference. Before editing, produce the
measurement the candidate's kind needs:

- Duplication and over-building: remove it and run `just check` — the compiler
  is the proof that nothing called it. For a dependency, `cargo check -p
  <crate> --all-targets`, the proof `tests/declared_dependencies.rs` says it
  is only a floor for.
- Unification: the merged shape passes `just check` with the layer headers
  unedited, or it is not a unification.
- A graph defect: `kern doctor --json > manifest.json`, read it, then
  `kern repair manifest.json` — repairs refuse while a daemon holds the
  writer lock, so the daemon stops first, with `just stop`. That recipe is
  `kern stop`, which reaches this root's daemon over its own socket and leaves
  every other project's daemon alone (stop-is-a-verb); `kern daemon` is
  restarted after.
- A model or ranking change: the before number and the after number from the
  same runner, both quoted.

5. Land

Small enough for one pass — the change lands here:

- The smallest diff that removes the excess. Deletion over addition; no shim,
  no alias, no compatibility layer kept behind it. Git is the archive.
- Non-trivial logic that survives the change keeps one runnable check, and a
  rule that must hold from now on is a Rust test under `tests/`, never a
  recipe or a hook (`gates-are-tests`).
- `just all` green — memos, check, and the full test suite. Nothing lands red.

Bigger than one pass — it becomes one `kind: work` memo, `level: 10`, in
`memos/intake/`, with a `Do` naming the files and a runnable `Check`.
[[work]] runs it later.

6. Record and loop

- A verdict worth keeping is one memo: `knowledge` for what the code actually
  does, `decision` for a candidate deliberately refused and why, `insight`
  for a connection across probes. One claim per memo in the same change
  (SYSTEM's atomicity law). Refusing a candidate is a result;
  record it so the next pass does not re-argue it.
- Author through `memo` writes; report results and warnings. One line: probe, candidate, what landed.
- Back to step 2 with the next probe. The loop ends when a full walk of the
  six yields nothing, or when the caller stops it.
