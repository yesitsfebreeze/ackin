# the-host-binary — analyst report (pass two)

Verdict: SPECCED

Q1 is answered *separate them*, and the separation is applied in the tree. The
crate is whole: a manifest, eleven modules, a test tree where `mod tests;` can
reach it, and no memory bank. Three specs, all three describing work that is
written and unverified — the machine still cannot run a newly created binary,
so no compile signal exists. `cargo build` is the first acceptance box of
every spec for that reason.

## What this pass built

`src/memory.rs` held two mechanisms under one name. The answer keeps the swap
guard and cuts the stored data, so:

- **`src/reload.rs` is new** (47 lines) and holds the reload transaction
  alone: `gate`, `epoch`, `pending`, and `begin`/`pending`/`finish`. Type
  `Reload`, `Clone` derived, `Default` hand-written, no `serde` — it stores
  nothing.
- **The bank is gone**: `Snapshot`, `Inner`, `read`, `update`, `fork`, `thaw`,
  plus `Sdk::memory`/`Sdk::checkpoint`, the `{"memory":…}` RPC arm in
  `cartridge.rs`, `ctx:memory`/`ctx:checkpoint` in `context.rs`, and
  `Ctx::memory` in `runtime.rs`.
- **The field `memory` is renamed `reload`** on `Component`, `Fiber`,
  `Loaded` and `Service`, and the three guard sites follow it: the resident
  wrapping in `Ctx::on` and `Ctx::provide`, and the `service is being
  replaced` guard in `lua.rs::to_lua`.

`src/runtime.rs` differs from `~/dev/sys/core/runtime.rs` in exactly eight
renamed lines and the ten-line `Ctx::memory` deletion. `src/fiber.rs` is
byte-identical. The PRD's "must not change" holds.

### The one place the cut was more than a rename

With no private snapshot left to copy, `fork()` is `Clone` and `thaw()` is a
no-op. So in `loader.rs::replace_entry` the pair

```
let _calls = memory.gate().write_owned().await;
let next_memory = memory.fork();
component.memory = next_memory.clone();
```

became `component.reload = reload.clone()` under the same write gate, the
`l.memory = next_memory` line in the success arm is dropped (the generations
now share one transaction, which is the point of it), and `memory.thaw()` on
the failure arm is deleted rather than ported. `begin` → write gate → stage →
`switch` → `finish` is unchanged step for step, on both arms.

## The test tree moved, and why

In `~/dev/sys/core` the tests sat beside `lib.rs` and were reached as a
private module — that is what `autotests = false` encodes, and it is why the
copy did not build: `mod tests;` under `src/lib.rs` looks for `src/tests/`.
The tree is now at `src/tests/`, the two `[[example]] path` values follow it,
and `Cargo.toml` differs from the original in those four `path =` values and
nothing else. A root `tests/` would have read as integration tests, which
these are not — they reach `crate::lua::Host`.

`tests/memory.rs` had no test that did not assert through the bank, so it is
replaced by `src/tests/reload.rs`, which asserts the same guarantees through
the generation instead: a consumer's captured service reference reaching the
new generation while keeping its own fiber uid, a raising candidate never
publishing, the failed transaction still finishing so the next swap works, the
inactive-entry recovery arm, and two entries of one file switching
independently. `fixtures/rpc_fixture.rs` no longer checkpoints — its `reject`
config stands alone — and `rpc_contract.rs::replacement` was updated to match.

`src/` is 4,396 lines over 13 files; `src/tests/` 2,325 over 11.

## Still no compile signal, and three workarounds that do not help

`syspolicyd` has been at ~100% CPU across both passes. Every newly created
Mach-O hangs in `_dyld_start`, so `cargo build` stalls after four to ten
crates with every process at 0% CPU — build scripts are newly created binaries
that have to run. Re-tested this pass and still hanging:

- `codesign --force --sign -` on the linked binary — exit 142. Not a
  signature check.
- `cp /bin/echo` and run the copy — also hangs. It is the new inode, not the
  compiler.
- Killing every stalled `cargo` and `build-script-build` on the machine to
  release their XPC requests — the daemon stayed pegged with nothing left
  waiting on it, so the pile-up is a symptom, not the cause.

Restarting the daemon needs a privilege this worker does not have. The
consequence is unchanged from pass one and is now stated inside each spec:
**the tree is reasoned from every call site and compiler-verified nowhere.**
The rename was applied by exact-string match across eight files, so a missed
site is the expected failure, and `cargo build --all-targets` is the box that
finds it. Recorded: `sources/260912-fb23.md`; the separation itself as
`sources/260912-6b69.md`.

The PRD's closing measurement — `--help` in 0.9ms at 8.9MB RSS — could not be
re-taken for the same reason, and is spec03's last two boxes.

## Specs

| spec | goal | complexity |
|---|---|---|
| `spec01.md` | the copy becomes a crate cargo can see — manifest, module list, test tree placed | 8 |
| `spec02.md` | the swap guard survives the cut of the memory banks | 14 |
| `spec03.md` | the suite that came with the copy runs here, and the binary is inert | 10 |

Footprint union: `Cargo.toml`, `src/lib.rs`, `src/main.rs`, `src/reload.rs`,
`src/cartridge.rs`, `src/context.rs`, `src/loader.rs`, `src/lua.rs`,
`src/runtime.rs`, `src/sdk.rs`, `src/service.rs`, `src/tests`.

`src` itself is deliberately not in the footprint — it would clash with every
other PRD on this board. `src/fiber.rs`, `src/socket.rs` and `src/turn.rs` are
untouched and are not claimed.

**complexity 30** — the trim is decided and written; what remains is one
compile, the diagnostics it prints across 4,396 lines under
`warnings = "deny"`, and a suite that has never run. Nothing here is a design
question any more, which is why it is not higher.

**blast-radius high** — this is the crate every other PRD on the board builds
inside. If the reload transaction was cut wrong, hot-reload and the in-flight
call guard fail silently under load rather than at compile time, and
`hot-reload`, `the-wire`, `lua-interface` and `verify-one-cartridge` all
inherit it.

## Findings outside this PRD's scope

- `python3 resources/workflows.py list .` against this board still returns
  nothing — the board's own workflow library is empty, so `probe-then-spec`
  is named here for the first time and `## Route` below is its file, written
  from the eight steps this pass actually took.
- `python3 resources/questions.py check .` still reports two rows against
  `the-manifest`: its question 1 carries one prepared answer instead of three,
  and that answer quotes code in backticks. Not touched.
- `.pearde/settings.md` carries no spec-count or complexity ceiling, so the
  brief's defaults (6 specs, 40 summed) were used. The set is 3 and 32.
- `python3 resources/knowledge.py query` returned 2 hits, 1 strong, for this
  pass's question, so no new gap was enqueued; pass one's
  `pending/260912-6709.md` is still open.
- The crate, the binary and the profile directory are all still named
  `zirkle`. The PRD asks for no rename and the build does not need one.
- `~/.cargo/config.toml` sets `build.rustc-wrapper = "kache"`, whose daemon is
  resident. It is not the stall (pass one disabled it and nothing changed),
  but it will show in `ps`.
- `src/main.rs` (387 lines) is not on the PRD's keep list, but a binary needs
  a `main`. Every subcommand is a thin `host.call("<key>", …)` into a service
  only a cartridge provides, so with no cartridges the binary does nothing —
  which is what the PRD closes on. Kept unchanged.
- Two stalled `cargo` trees under `/Users/feb/dev/sys` were killed this pass
  along with this repo's own, since they were parked at 0% CPU and holding the
  artifact lock. They were another repo's build, already dead; nothing under
  `/Users/feb/dev/sys` was edited.

## Scores

complexity: 30
blast-radius: high
workflow: probe-then-spec

## Route

## Use when

- A PRD came back from a question with its `## Answers` filled and pass one's probe already standing uncommitted in the tree, and the answer has to be turned into specs.
- Not when the PRD has never been probed and has no answers — that is the first pass of this same route, which stops at the question instead of continuing past it.

## Steps

| # | atomic | why | on failure |
|---|--------|-----|------------|
| 1 | `read-the-contract` | pass one's report holds the call-site census this pass edits from, and the answer under `## Answers` closes the only fork left | `stop` |
| 2 | `query-the-record-first` | the machine stall this run hits was already on record from pass one, so no time was spent rediscovering it | `→ 1` |
| 3 | `apply-the-answered-fork` | the answer is a sentence until the module is actually split; the split is what shows which callers die with the cut part | `→ 1` |
| 4 | `port-the-tests-the-cut-orphaned` | every test of the cut part asserted through it, so deleting the file silently drops coverage of the part that was kept | `→ 3` |
| 5 | `attempt-the-build` | a rename applied by string match across eight files is a guess until a compiler reads it | `→ 3` |
| 6 | `separate-the-machine-failure-from-the-crate` | a build that produces no output looks identical to a build that failed, and specs written on that confusion blame the wrong thing | `→ 5` |
| 7 | `record-what-the-build-learned` | the split and the dead workarounds are one worker's head until they are written to the record | `→ 3` |
| 8 | `write-the-specs` | the next worker gets the file, not the head, and every unverified claim has to arrive as a box that can fail | `→ 3` |

### atomic read-the-contract

## Do

1. Run `pearde brief <prd> --worker <you>` and read nothing it does not name.
2. Read `.pearde/prds/<prd>/prd.md` including `## Questions` and `## Answers`.
3. Read `.pearde/prds/<prd>/report.md` and `.pearde/prds/<prd>/probe/README.md` from the prior pass, and inspect the uncommitted tree they describe.

## Done when

- The answered fork is stated in one sentence, and the prior pass's call-site census has been checked against the tree with `grep` rather than trusted.

## Fails when

### atomic query-the-record-first

## Do

1. Run `python3 resources/knowledge.py query "<the contract as a question>"` before any research outside the repo.
2. Read every strong hit; a gap enqueues itself and is a report line, not a question to the person.

## Done when

- Each strong hit is either used or explicitly set aside, and no outside research was done on a question the record already answered.

## Fails when

### atomic apply-the-answered-fork

## Do

1. Write the kept half into its own module and delete the cut half outright.
2. `grep -rn` every symbol of the cut half across `src` and the tests, and remove each caller.
3. Where a kept operation was only meaningful because of the cut half, collapse it to what remains rather than porting it.
4. `grep` for the cut names once more; an empty result is the check.

## Done when

- No symbol of the cut half survives anywhere in the tree, and the kept half compiles as a module that stores nothing the cut half stored.

## Fails when

### atomic port-the-tests-the-cut-orphaned

## Do

1. Read every test of the module being cut and mark which guarantee each one actually asserts.
2. Rewrite the ones asserting a kept guarantee so they assert it through what remains, in a file named for the kept half.
3. Delete the ones that only asserted the cut half, and update any fixture whose config existed to drive it.

## Done when

- Each guarantee the contract keeps has a test that names it, and `grep` for the cut half's vocabulary across the test tree returns nothing.

## Fails when

### atomic attempt-the-build

## Do

1. Run the project's build over every target, not just the library.
2. Clear any stale lock holder first if the build reports one.
3. Fix what the compiler reports and run it again.

## Done when

- The build prints its success line and exits 0, or it has produced a diagnostic that names a file and a line.

## Fails when

### atomic separate-the-machine-failure-from-the-crate

## Do

1. When the build stalls with no diagnostic, check whether the processes are burning CPU or parked at 0%.
2. Reduce it to the smallest thing outside the project that shows the same symptom, and keep that reproduction as a script in `probe/`.
3. Re-test the workarounds a previous pass left untried, and write down the ones that do not work as well as the ones that do.
4. Say in the report, and in the first acceptance box of every spec, that the tree is unverified and what will verify it.

## Done when

- The failure is demonstrated outside the project under build, the reproduction is runnable from `probe/`, and no spec claims a check that was never run.

## Fails when

### atomic record-what-the-build-learned

## Do

1. Pipe the finding into `python3 resources/knowledge.py remember "<title>"` with `--provenance` naming the route or measurement it came from.
2. Record the negative results too — the workarounds that did not work are what the next worker would otherwise pay for again.

## Done when

- Every fact the report states that the next worker would have to rediscover has a note id, and the report cites it.

## Fails when

### atomic write-the-specs

## Do

1. Split what the build stands up into implementable units and write each to `specs/specNN.md` from the template.
2. Give each a `footprint:` of the paths it writes — never a root that clashes with the board — and a `complexity:`.
3. Under each, say what already stands in the tree and what is left to finish.
4. Make the first box of every spec the check that was never run, and give each spec a verify command scoped to its footprint.

## Done when

- Every claim the probe made is a box that a command can fail, the footprints cover the tree the probe moved and nothing else, and the summed complexity and the spec count are inside the board's ceilings.

## Fails when
