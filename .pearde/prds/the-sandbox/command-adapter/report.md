Verdict: DONE

# Command adapter verified

`specs/spec01.md`: 6/6 acceptance boxes closed. The source exports one
manifest-derived standard command constructor, and the asynchronous wrapper
uses that same constructor with piped stdio and kill-on-drop. Empty input is
refused. Linux remains an explicit Unsupported stub. Existing callers are
unchanged, as required by this prerequisite contract.

No source change was needed after the analyst implementation. Recovering
space for generated build artifacts resolved the previous fixture failures.
No files in this footprint were under the board health floor. The scope is
three files: `src/lib.rs`, `src/sandbox.rs`, `src/sandbox/linux.rs`.

## Verify and Proof

Ran from the child lane with CARGO_INCREMENTAL=0 throughout:

```
CARGO_INCREMENTAL=0 cargo test --all-targets
Finished test profile [unoptimized + debuginfo] target(s) in 4.45s
127 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 7.17s
```

The binary and all four example test targets also exited zero with zero tests.
The complete output is `/tmp/cartridge-command-adapter-full-final.log`.

```
CARGO_INCREMENTAL=0 cargo test --lib sandbox
8 passed; 0 failed; 0 ignored; 0 measured; 119 filtered out; finished in 0.46s
```

The scoped checks exercise empty-input rejection, real synchronous and
asynchronous outside-write denial, and five existing policy compiler cases.
The Linux stub was inspected directly: its entire function returns
`Err(std::io::Error::new(std::io::ErrorKind::Unsupported, ...))`; the Linux
cfg branch forwards that error. This establishes refusal, not Linux confinement.

The existing record [[260912-b09f]] identified the previous disk failure.
The successful rerun was recorded through knowledge.py with actual commands
as provenance. No outside research was needed. The user's separate scored
critique gate remains the orchestrator's review of this candidate; these
passing tests are not invented performance, cleanliness or size scores.

## Workflow probe-then-spec

| # | atomic | outcome | note |
|---|---|---|---|
| 1 | read-the-contract | passed | child PRD, all six boxes, previous report, and current constructors inspected |
| 2 | query-the-record-first | passed | prior disk-failure note used |
| 3 | apply-the-answered-fork | passed | constructor already extracted; no remaining source change in this contract |
| 4 | port-the-tests-the-cut-orphaned | passed | existing tests retained; no cut module or orphaned test |
| 5 | attempt-the-build | passed | all targets pass, 127/0; scoped sandbox 8/0 |
| 6 | separate-the-machine-failure-from-the-crate | passed | Finished test profile in 4.45s; machine-failure work not applicable after space recovery |
| 7 | record-what-the-build-learned | passed | successful rerun recorded with command provenance |
| 8 | write-the-specs | passed | existing spec retained, boxes closed as checks completed, output attached |

## Skeptic consultation

KEEP for this limited API addition: existing callers unchanged; synchronous and asynchronous denial tests check successful shell completion, denied output, and absent outside file. Linux refusal is inspected, not Linux runtime proof. Report and all six acceptance boxes reconciled with final 127-pass log before collect.
