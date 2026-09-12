---
kind: routine
description: Answer one tooling question as a measurement — phrase the job as a choice, rank the candidates on pearde's scout routes, measure the decisive axis on this tree, land the verdict as one decision memo.
uses:
  - usage: "[[run-usage]]"
    when: [picking a tool, or answering \"is X worth it\" with a number]
---

1. Read

- SYSTEM (`memos/SYSTEM.md`) for the decision block and the index.
- method — how a claim is probed here; `one-reading-is-not-a-measurement`
  and `read-twice-by-different-means` govern every number this routine
  reports.
- `~/dev/infra/pearde/resources/scout/README.md` — the scout itself: seven
  verbs, four layers, the research loop. `findings.md` beside it is what
  pearde already measured; a job answered there is read, not re-run.

2. Phrase the job as a choice

One sentence naming what is being picked and what it must beat — "several
sessions on one Rust tree without N full build dirs", never "is X good". A job
with nothing to reject has nothing to measure. Check
`~/dev/infra/pearde/resources/scout/findings.md` for the same job first.

3. Rank the candidates on at least two axes

The scout's routes, run from its own directory so `$HERE` resolves:

```
cd ~/dev/infra/pearde/resources/scout
./scout.sh find gh 'cargo-sweep OR kondo OR sccache language:rust stars:>100'   # stars, last push, state
./scout.sh find crates '<crate>'                                                # recent downloads
./scout.sh find brew '^sccache$|^kondo$'                                        # installs on real Macs
./scout.sh find list                                                            # every other route
```

Two axes that disagree are the finding — distro breadth with no installs is
an abandoned tool. `gh` alone is an opinion.

4. Measure the decisive axis on this tree

The ranking says what is alive; only a number from this tree says what
works here. Reproduce the cost the job names — disk, seconds, files — with
one instrument, then confirm with a second of a different kind (a `du`
against a `df` delta, a `find` count against a Python `scandir`). A
`cargo` experiment runs in a scratch worktree with its own
`CARGO_TARGET_DIR`, never in the shared tree:

```
git worktree add --detach "$SCRATCH/lane" HEAD
CARGO_TARGET_DIR="$SCRATCH/lane/target-<mode>" cargo test --no-run
git worktree remove --force "$SCRATCH/lane"
```

5. Write the verdict

One `kind: decision` memo in `memos/intake/`, the decision block from
SYSTEM: the pick, what it beat, the numbers with the route or command
that produced each, the date, and what would overturn it. A pick standing on
one axis says `status: unmeasured`. Anything the job asked and did not
answer becomes its own `kind: question` memo for [[drill]].

6. Close

- Author through `memo` writes and report saved/index status and any warnings.
- Report one line: the job, the pick, the one number that decided it.
