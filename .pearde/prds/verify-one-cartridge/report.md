Verdict: DONE

# verify-one-cartridge — pass two, implementer quill, 2026-09-12

This pass was **close-the-boxes, not re-split**: `specs/` already held
`spec01.md` from pass one and the contract had not changed, so per the
workflow's step-4 Do 0 no new units were written — every box was closed with
the command that closed it and its output, ticked as it closed.

## Workflow drive-the-binary

| # | step | outcome |
|---|------|---------|
| 1 | read-the-contract | done — census re-checked against the lane tree by `grep`, not trusted from the report: `Command::Verify { cartridge }` at `src/main.rs:100/775`, `Host::solo` / `verify_one` / `solo_entry` / `run_contracts` in `src/loader.rs` (719-950), 12 tests in `src/tests/contracts.rs`. The probe script was run before anything was believed, and the tree was inspected: the work lives on `lane/verify-one-cartridge`, clean, rebased onto `main` |
| 2 | attempt-the-build | done — `touch src/main.rs` then `cargo build`: `Compiling zirkle` appeared, `Finished dev profile in 1.94s` (no cache-hit dodge; `rustc-wrapper = kache` is set, which is why the touch was needed). `cargo test`: 119 passed, 0 failed. `cargo test --lib tests::contracts`: 12 passed, 0 failed, every test the spec names among them |
| 3 | probe-the-loop-end-to-end | done — `probe/verify-one-cartridge.sh` run from the repo root, exit 0, all seven steps walked with exit codes on the record: pass line + exit 0; failing neighbour kept out; failing contract named (path, obligation, key) + exit 1; unbound need named before load; unknown cartridge an error; armless `verify` unchanged; `ledger` unchanged. A step the probe did not cover — the ambiguous need — got its own fixture and was walked against the binary too (see Edits) |
| 4 | write-the-specs | closed-the-open-boxes variant (specs pre-existed, contract unchanged). `specs/spec01.md`: 8/8 boxes `[x]`, each with the closing command and its output. Summed complexity 8, one spec, inside the board's ceilings |

### Edits

- The probe did not walk the ambiguous-need box (`store: need \`provider\` is
  ambiguous (provider, twin)`), so pass two added
  `probe/ambiguous-need.sh` — provider + twin of one scope, `verify store`
  prints the line and exits 1, neither side loaded. The box quotes it.
- The spec's "cargo test runs 82 tests" is stale: after the lane was rebased
  onto `main` the count is 119 (0 failed). Ticked with the real number; the
  twelve named tests are all present and green. The spec text was left as
  written — the box annotation carries the correction.
- No workflow step failed; no back-edge was taken.

## The repo's own gate

- `cargo test` over the lane: green (119 passed).
- `cargo clippy --all-targets`: fails on `main` itself, not on this lane's
  work — `src/tests/wire.rs:163:14`, `unnecessary use of to_owned`
  (`host.call(&key.to_owned(), json!(null))`, help: use `key`). That file came
  in from `the-wire`'s work (`ab7930e`), is outside this spec's footprint, and
  fails identically in the root checkout, so it is reported here, not fixed.
- `cargo fmt --check`: 344 diffs across the root checkout, spread over nearly
  every file under this toolchain (rustfmt 1.8.0 / rustc 1.94.0) — the tree
  has never been fmt-clean as a whole, so fmt is not a gate this board runs.

## Health floor

Footprint named no files under the floor, and nothing moved inside the
footprint: the lane's code stands from pass one and every box was a landing
check, so nothing needed improving and nothing was refactored.

## The record is still unreachable

`resources/knowledge.py` (and `workflows.py`, `grammar.py`) exist nowhere in
the repo — re-confirmed this pass with `ls` and `find` over the whole tree.
The brief's `query`/`remember`/`enqueue` steps could not run; the gate finding
(clippy on `src/tests/wire.rs`) is the fact this pass would have enqueued, and
it stands here by hand because there is no script to enqueue it with. Second
pass in a row to pay this.

## Carried finding, unchanged from pass one

Where a failing contract's teeth belong beyond the human loop (exit code only,
a machine-written stamp, or verify-at-launch) is still the board's decision —
the bind/launch ground is `the-resolver`'s claimed footprint and its PRD is
silent about verification. Nothing in this pass changed that; the exit-code
reading is what stands and what the boxes certify.

## Files this pass touched

- `specs/spec01.md` — 8 boxes ticked with commands and output
- `probe/ambiguous-need.sh` — new, executable, walked
- `report.md` — this file

No source files changed: the lane's committed code passed every check as it
stood.
