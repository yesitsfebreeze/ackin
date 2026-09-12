# the-host-binary — implementer report (pass three)

Verdict: BLOCKED

The machine still has no compile signal. `probe/dyld-hang.sh` prints `exit=142`
and `syspolicyd` pid 498 is at 100.0% CPU with 129 minutes of CPU time — the
same pid, unbroken, across all three passes. The remedy is still the one the
user holds and has not run: `sudo killall syspolicyd` (with `sudo killall
amfid`).

Every box that a grep, a diff or a file-structure fact can close is closed and
ticked: **7 of 15**. The remaining 8 are all compile boxes or run boxes and
they are open, untouched, and honestly unticked. Nothing in this report claims
a build, a test or a measurement that was not run.

This pass did not merely re-confirm the wall. It found the mechanism, ruled out
the one workaround that looked most promising, and left both on the record so
the next worker does not pay for either again.

## The one question for the user

**Run `sudo killall syspolicyd && sudo killall amfid`.** launchd respawns both
immediately; no reboot, no lost session. Then `sh
.pearde/prds/the-host-binary/probe/dyld-hang.sh` must print `exit=0`. Every
open box below is a single `cargo build --all-targets` and `cargo test` away
once it does. There is no other route — see "Why there is no way around it".

## Box status

| spec | ticked | open | what is open |
|---|---|---|---|
| spec01 | 2 / 4 | 2 | `cargo build --all-targets` finishes; build emits no warning |
| spec02 | 5 / 6 | 1 | `cargo build --all-targets` exits 0 |
| spec03 | 0 / 5 | 5 | `cargo test`; `reload.rs` passes; `--help` exits 0; the 3ms/16MB measurement; the binary is inert |

### spec01 — ticked

- **`src/lib.rs` names no module that has no file, and no file under `src/` is
  unreachable from it.** 11 modules listed, 11 files present. The only files
  under `src/` that are not modules are `lib.rs` (the lib root) and `main.rs`
  (the `[[bin]]`).
- **`Cargo.toml` differs from `~/dev/sys/core/Cargo.toml` only in the four
  `path =` values.** The normalised diff is empty; the raw diff is exactly
  lines 12, 16, 21 and 26.

### spec02 — ticked

- **No symbol of the cut half survives.** `grep -rn
  "Snapshot\|checkpoint\|\.thaw()\|\.fork()" src` is empty, and so is `grep
  -rni "\bmemory\b\|development\|landscape" src`.
- **`src/reload.rs` stores nothing.** Fields are `gate`, `epoch`, `pending` at
  lines 12-14. 47 lines, no `serde`.
- **`src/loader.rs` keeps the transaction step for step.** `begin()` L821,
  `gate().write_owned()` L854, `finish()` at L849 (prepare-failure arm), L881
  (success arm) and L896 (switch-failure fallthrough). Read the body at
  L818-896 to confirm it, rather than trusting the grep.
- **`src/fiber.rs` is byte-identical** to `~/dev/sys/core/fiber.rs` — `diff`
  empty.
- **`src/runtime.rs` differs only in renames plus the `Ctx::memory`
  deletion** — see the correction below.

### spec03 — nothing ticked

The grep half of box 2 passes (`grep -rni "bank\|snapshot\|checkpoint\|memory"
src/tests` is empty, 2,325 lines over 11 files as specced), but the box also
requires `src/tests/reload.rs` to *pass*, which needs a test run. A compound
box cannot be half-ticked, so it stays open.

## A spec claim that is off by one

`spec02` acceptance box 6 read *"differs … in exactly 8 renamed lines"*. It is
**7**, not 8: lines 59, 71, 122, 150, 477, 579, 583, plus the 10-line
`Ctx::memory` deletion at L387-396, and nothing else. The substance of the box
holds exactly — only renames and that one deletion — so the box is ticked with
the count corrected in place. The PRD's "must not change" on `runtime.rs` and
`fiber.rs` holds.

## The build could not be run, and this is why

`probe/dyld-hang.sh`, run at the start of this pass:

```
exit=142  (0 healthy, 142 = hung in dyld)
```

```
root  498  100.0  /usr/libexec/syspolicyd    129:49.75
```

### The mechanism, which was not known before this pass

The rule on record was "every newly *linked* Mach-O hangs". That is not quite
it, and the sharper version explains everything the previous passes saw. **The
dyld code-signing verdict is cached per inode, not per path.** New probe,
`probe/dyld-inode.sh`, on a known-good pre-peg binary:

```
original path+inode : exit=0
hardlink same inode: exit=0
copy    new inode  : exit=142
```

A hardlink of a working binary runs. A copy of the same binary does not. It is
the new inode that has no verdict, not the linker.

This also explains a stall that looks like something else entirely. `sample(1)`
on the stalled `rustc` shows it is **not** hung at process start — it reaches
`main`, loads `librustc_driver`, enters `run_compiler`, and blocks here:

```
run_compiler -> dlopen_from -> Loader::getLoader -> makeJustInTimeLoaderDisk
  -> Loader::mapSegments -> SyscallDelegate::fcntl -> __fcntl
```

That `fcntl` is the signature check on a **proc-macro dylib being dlopen'd**.
So a live `rustc` sitting at 0% CPU is this, not a jobserver deadlock. Worth
knowing before anyone spends an afternoon on the wrong hypothesis.

### Why there is no way around it

The obvious workaround is to point `CARGO_TARGET_DIR` at a target directory
built before the peg, so cargo reuses cached artifacts and executes nothing
new. Tested this pass against `~/dev/sys/target`, and it is the closest anyone
has got:

- It **works, partially.** The build ran well past the ~10-crate wall the
  previous passes hit and reached the heavy leaf crates — `tokio`, `mlua`,
  `rustix`, `tempfile`. Build-script *outputs* are reused without re-executing
  the scripts, so that entire class of stall disappears.
- It **still dies**, at the first `dlopen` of a proc-macro dylib.
- Hardlinking the cached dylibs in so they carry their verdicts (96 of them,
  inodes confirmed matching) **does not help**, because every proc-macro dylib
  in that warm tree was itself created during the peg and so never had a
  verdict to cache:

```
dylibs in ~/dev/sys/target/debug/deps created during the peg: 31
dylibs predating the peg (would carry a verdict):              0
```

`probe/warm-cache-check.sh` answers that in one command. Run it before
attempting this route; if it reports zero pre-peg dylibs, the route is closed
before you start.

Also ruled out: **removing the `RUSTC_WRAPPER`**. `~/.cargo/config.toml` sets
`rustc-wrapper = "kache"`, and a wrapper in front of every `rustc` was a
plausible suspect. It is not — `kache` is an old binary with a cached verdict,
and with it disabled the build stalls at the identical point.

Together with the three the previous pass killed (`codesign --force --sign -`,
`cp` of a working binary, killing stalled `cargo` processes), **every non-root
avenue is now closed with evidence.** There is no compile signal on this
machine without restarting the daemon.

### Left clean

The 83 GB clone used for the experiment was copy-on-write and cost no disk; it
and every process this pass started have been removed. Free space is where it
started and the lane tree is clean — `git status --short` is empty, so the
probe's work is intact as committed.

## On the record

Both findings are written to the knowledge base, cited rather than left
standing only here:

- `[[260912-7adb]]` — the hang is keyed by inode, so a hardlink runs and a copy
  does not; includes the trap that a binary *created* during the peg hangs from
  its own inode and looks like a counter-example. That trap cost one wrong
  conclusion in this pass before it was caught.
- `[[260912-9edf]]` — a warm cargo target dir does not restore a compile
  signal; how far it gets, the `dlopen` stack where it dies, the one-command
  pre-check, and the `RUSTC_WRAPPER` dead end.

Both refine `[[260912-7a41]]` and `[[260912-ebde]]`, which the record already
held and which were read first. No research outside the repo was done — step 2
returned three strong hits and they covered the question, so the only new work
was measurement.

## The standing direction: `pub` items nothing in this tree reaches

Swept `src/` for `pub fn`s with no caller anywhere in the tree, tests included.
Exactly one:

- **`sdk::on_reload`** (`src/sdk.rs:58`) — *"Cooperatively yield long-running
  calls before this cartridge is replaced."* No caller in `src/` or
  `src/tests/`. It is public SDK surface a cartridge process would call, not
  dead host code, so it is **reported and not deleted** — which is what the
  board note asks for `pub` items the compiler will never flag.

Worth a look when it is next touched: `Sdk` has a field named `reload` holding
a map of handlers, while `crate::reload::Reload` is the transaction type. The
two are unrelated and now share a word. Not a defect, and not in scope here.

The private unreachable items are the compiler's job — `warnings = "deny"` will
name them the moment a build runs, and that is one of the open boxes.

## Health

No health record existed; `pearde health score` wrote one. In footprint, worst
first:

| file | score | flags |
|---|---|---|
| `src/loader.rs` | 30 | branching, lines |
| `src/main.rs` | 58 | longest, lines |
| `src/cartridge.rs` | 66 | lines, branching |
| `src/tests/process.rs` | 68 | lines, longest |
| `src/lua.rs` | 69 | lines, branching |
| `src/runtime.rs` | 72 | lines, longest |

**Nothing moved, and nothing could.** Every one of these is flagged for size
and branching, so the only improvement is a split — which the contract names a
defect outside scope, to be reported and not done. Beyond that: `runtime.rs` is
under the PRD's explicit "must not change", and with no compiler on the machine
any edit to the other five would be unverifiable, which is precisely what this
report refuses to do. `src/reload.rs`, the file this PRD actually created,
scores 100.

## Workflow probe-then-spec

| # | step | outcome |
|---|---|---|
| 1 | read-the-contract | done — PRD, `## Answers`, pass-two report and `probe/README.md` read; the census re-checked with `grep` rather than trusted |
| 2 | query-the-record-first | done — 3 strong hits, all used, no outside research |
| 3 | apply-the-answered-fork | already standing from pass one; verified empty on every cut symbol |
| 4 | port-the-tests-the-cut-orphaned | already standing; `grep` for the cut vocabulary across `src/tests` returns nothing |
| 5 | attempt-the-build | **blocked** — the build does not fail, it hangs; one probe run is the whole test and it printed 142 |
| 6 | separate-the-machine-failure-from-the-crate | done — mechanism found, reproduction runnable from `probe/`, two workarounds newly ruled out |
| 7 | record-what-the-build-learned | done — `[[260912-7adb]]`, `[[260912-9edf]]`, both negatives included |
| 8 | write-the-specs | not re-entered; specs stood from pass two and were ticked, not rewritten |

The answered fork in one sentence: **Q1 is *separate them* — `src/memory.rs`
split into `src/reload.rs` (gate, epoch, pending) and a deletion of the bank,
with the swap guard kept and the stored data cut.**

No back-edge was taken. Step 5 is a machine wall, not a step failure, so it
routes to step 6 by its own text rather than back to step 3.

### Edits

Replacement text for the places the atomics misled this run.

**1. Step 5 `attempt-the-build`, item 2.** Reads *"Clear any stale lock holder
first if the build reports one."* The build never reports one — under this
failure it reports nothing at all, and the actual hazard is the opposite:
prior runs leave parked processes that the next run inherits and that never
appear in any message. 22 stale `build-script-build` processes from earlier
passes were still resident when this pass started. Replace with:

> 2. Clear any stale lock holder the build reports. If the build reports
>    nothing at all, check for parked processes from previous attempts
>    (`ps -eo pid,stat,time,comm | grep -E 'rustc|cargo'`) and kill them before
>    re-running — a hung toolchain leaves children that no message names.

**2. Step 5 `attempt-the-build`, "Done when".** Reads *"The build prints its
success line and exits 0, or it has produced a diagnostic that names a file and
a line."* Neither disjunct can be satisfied by a build that hangs, so the step
has no exit and the worker cannot tell "keep waiting" from "stop". Replace
with:

> - The build prints its success line and exits 0, or it has produced a
>   diagnostic that names a file and a line, or it has been shown to hang
>   rather than fail — in which case go to step 6 and do not re-run it.

**3. Every atomic's `## Fails when` is empty.** All eight in this workflow have
the heading and no entries. The contract says a report must carry *"the
replacement text for every failure the atomic caused — … a shape `## Fails
when` does not list"*, but a section that lists nothing cannot not-list a
shape, so the check cannot fail and the instruction is unrunnable as written.
Either populate the sections or drop the heading; as it stands it reads as an
omission at every step and a worker cannot tell which.

**4. Step 6 item 3, "Re-test the workarounds a previous pass left untried".**
This one earned its keep — it is what produced both knowledge notes — but it
has no stopping rule, and workaround space is unbounded. Suggest appending:

> …and stop when the remaining candidates all require a privilege you do not
> have; say so and name the privilege rather than continuing.

## What is left for the next worker

Nothing but the compile. The tree is written, the specs are ticked to the
limit of what can be checked without a compiler, and the two knowledge notes
mean none of this pass's diagnosis has to be repeated. After `sudo killall
syspolicyd && sudo killall amfid`:

```sh
sh .pearde/prds/the-host-binary/probe/dyld-hang.sh    # must print exit=0
cargo build --all-targets 2>&1 | tail -20             # spec01, spec02
cargo test 2>&1 | tail -30                            # spec03
./target/debug/zirkle --help                          # spec03
```

With `warnings = "deny"`, expect the first build to surface orphaned imports
and now-dead private functions from the cut — that is the compiler doing the
half of the standing direction that grep cannot.
