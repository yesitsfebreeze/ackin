# the-build-signal — implementer report

Verdict: DONE

spec01 — 7 of 7 boxes ticked, every one on a command run here. The gate
`.pearde/prds/the-build-signal/probe/build-signal.sh` exits 0 against the repo
root `/Users/feb/dev/cartridge` and prints `signal=live`; its output is in
`probe/signal.log`. The fault this PRD was opened against is gone and the board
has a compiler it can name.

No code was written. The only file changed outside the report and the spec is
the probe script itself, inside the footprint, for the counting defect under
`## What the run found`.

## Boxes

| box | evidence |
|---|---|
| exits 0, last line `signal=live` | `gate-exit=0`; `tail -1 probe/signal.log` prints `signal=live` |
| four stages `OK` | `dyld OK` / `cargo-build OK` / `zirkle-help OK` / `cargo-test OK (53 passed)` |
| `cargo-test` passed count above zero | `53 passed`, cross-checked against cargo's own `test result: ok. 53 passed; 0 failed` |
| no stage reached by waiting | cap proven live: `perl -e 'alarm 2; exec(@ARGV)' sleep 60` returns rc=142 after 2s; stage 1 rebuilt round a 300s-sleeping binary prints `dyld HUNG rc=142` and exits 1 after 3s. Whole green run: wall=5s |
| pid 498 gone, `syspolicyd` under 5% | `ps -p 498` exit 1, no process; pid 93109 at 0.0% CPU, started Sat Sep 12 16:11:51 |
| output written to `probe/signal.log` | file present, header line carries host, time, wall and exit |
| nothing outside the PRD dir modified | `git status --porcelain` names `?? .pearde/prds/the-build-signal/` and `?? .pearde/wiki/sources/260912-ea92.md` (the record note), nothing else |

## Verify output

```
# build-signal gate, repo root /Users/feb/dev/cartridge, 2026-09-12 16:28:13 CEST, wall=5s, exit=0
dyld                   OK
cargo-build            OK
zirkle-help            OK
cargo-test             OK (53 passed)
signal=live
```

## What the run found

**The gate can fail, and was made to.** A green gate that cannot go red proves
nothing. Pointed at a crate that is not there it prints `cargo-build FAILED
rc=101`, `zirkle-help HUNG or missing`, `cargo-test FAILED rc=101`,
`signal=dead`, exit 1. The caps fire at their alarm rather than at the wall.
Both demonstrations are in the record note, not only here.

**A counting defect in the gate, fixed inside the footprint.** The first run
against the repo root reported `52 passed`; `cargo test` reports 53 on three
consecutive direct runs. The stage counted with `grep -c '^test .* ok$'`, and
libtest writes those per-test lines from several threads, so they interleave and
a line occasionally loses its prefix or its trailing ` ok`. The stage now sums
the per-binary summary lines:

```sh
n=$(sed -n 's/^test result: ok\. \([0-9][0-9]*\) passed.*/\1/p' /tmp/bs-test.$$ |
    awk '{t+=$1} END{print t+0}')
```

Stable at 53 across four gate runs after the change, and it still fails closed —
a log with no summary line sums to 0 and the stage reports `FAILED 0 passed`.
This is a one-line correctness fix to the file the spec's own box reads, not a
refactor. **Anything else on this board that gates on a test count scraped from
libtest stdout has the same bug**; I did not go looking outside this footprint.

**Census re-checked against the tree, not carried forward.** Pass one's claim of
8 open boxes on [[the-host-binary]] holds: `grep -c '^- \[ \]'` gives 2 in
spec01, 1 in spec02, 5 in spec03. None ticked from here — they are that PRD's
work. Three of them would go green on this host today.

**Health floor.** The brief named no file under the floor, and the one file
touched is a 50-line shell script well above it. Nothing moved.

## Record

[[260912-ea92]] — "the build signal is live on this host and the gate that says
so is falsifiable" — carries the six green runs, the two falsification
demonstrations, and the libtest counting defect with its replacement snippet.
Written to `.pearde/wiki/sources/` at the repo root, not in the lane.

Queried before any outside research: 9 strong hits, none needing a second
source. [[260912-854a]] already held this pass's whole answer — the daemon
respawned itself under pid 93109 and `amfid` was never part of the fault — so no
time went on rediscovering it. [[260912-fb23]] ruled out the workarounds I would
otherwise have retried. No gap was enqueued and no research happened outside the
repo.

## Defects outside scope, reported and not fixed

- `prd.md`'s `## What closes this PRD` still prescribes `sudo killall
  syspolicyd && sudo killall amfid`. The `amfid` half is not load-bearing —
  pid 336 has run unchanged since 2026-09-11 13:00:08 and predates the peg.
  The correction is on the record as [[260912-854a]]; the conclusion
  [[a-pegged-syspolicyd-is-a-machine-fault-that-looks-exactly-li]] should be
  re-read against it. Not edited: this PRD writes nothing in the tree.
- Q1 in `prd.md` is moot — none of its three options happened, a fourth outcome
  did. Left untouched, as pass one left it. Closing this PRD retires the
  question with no edit.

## Workflow probe-then-spec

| # | step | outcome |
|---|---|---|
| 1 | read-the-contract | done — prd.md, pass one's report, spec01 and the probe dir read; the census re-grepped against the tree (2+1+5=8) rather than trusted |
| 2 | query-the-record-first | done — queried before anything else, 9 strong hits, two read in full and used, no outside research, no gap enqueued |
| 3 | apply-the-answered-fork | not applicable — see Edits. There is no module to split; this PRD's contract forbids writing code |
| 4 | port-the-tests-the-cut-orphaned | not applicable — nothing was cut, so nothing was orphaned. The crate's 53 tests all still run |
| 5 | attempt-the-build | done — every target, not just the library: `cargo build`, the linked binary's `--help`, and `cargo test`. Success line, exit 0, six times. No parked `rustc`/`cargo` before or after |
| 6 | separate-the-machine-failure-from-the-crate | not entered — step 5's `Done when` was met on its success branch. The machine failure is already separated, on the record, and gone |
| 7 | record-what-the-build-learned | done — [[260912-ea92]], including the negative result (the libtest count) and the falsification runs, at the repo root |
| 8 | write-the-specs | already satisfied by pass one — spec01 existed and was implemented here rather than rewritten. Its boxes are ticked on commands, and its first box was the check that had never been run |

### Edits

**Steps 3 and 4, `## Use when` and both `## Fails when` blocks.** The route
assumes every answered fork is a module cut: step 3 says "write the kept half
into its own module and delete the cut half", step 4 says "every test of the cut
part asserted through it". This PRD's fork was a machine-privilege question —
restart a daemon, reboot, or move the build elsewhere — answered by events when
`launchd` respawned `syspolicyd`, and its contract says "Must not change:
nothing in the tree. This PRD writes no code." A worker following the atomic
literally would have to invent a split to have something to cut. `## Fails when`
lists no shape for this, so the failure it does list ("the answer is restated in
prose and the module is left whole") reads as a hit when it is not one.
Replacement, as step 3's first `## Fails when` bullet:

> - The answer is restated in prose and the module is left whole. Until the
>   split exists, which callers die with the cut part is a guess. This bullet
>   binds only when the answered fork names code; a fork answered about the
>   machine, the environment or a privilege has no cut half, and the step is
>   recorded as not applicable with the contract line that says so quoted.

**Steps 2 and 7, the `knowledge.py` path.** Both name
`python3 resources/knowledge.py`, which resolves to nothing from the board's
repo root — `/Users/feb/dev/cartridge/resources/` does not exist. The file is at
`/Users/feb/dev/infra/pearde/resources/knowledge.py`, alongside the
`workflows.py` the brief itself calls by absolute path. Replacement for the
command in step 2 `Do` 1:

> 1. Run `python3 <pearde>/resources/knowledge.py query "<the contract as a
>    question>"` from the board's repo root, where `<pearde>` is the install
>    directory the brief's own `workflows.py` line names, before any research
>    outside the repo.

**Step 2, `Do` 2, "Read every strong hit".** `knowledge.py` has no verb that
prints a note — `show` is not in `{remember, conclude, enqueue, query, relink,
board, index, wiki, dashboard, doctor, harvest, round}`, and the query output
gives slugs, not paths. Replacement:

> 2. Read every strong hit at `.pearde/wiki/<type>/<slug>.md` — `query` prints
>    the slug and its folder, not the text; a gap enqueues itself and is a
>    report line, not a question to the person.

**Step 5, `Do` 1, "the project's build".** On a Rust board `cargo build` alone
leaves the test and bin targets unlinked, which is exactly where the fault this
route was written for shows up. Replacement:

> 1. Run the project's build over every target, not just the library — for a
>    Rust crate that means the binaries link and `cargo test` runs, not only
>    `cargo build` or `cargo check`.
