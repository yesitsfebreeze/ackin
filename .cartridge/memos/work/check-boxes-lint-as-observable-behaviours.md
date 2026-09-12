---
kind: work
description: "The memo write validates Check boxes as command-observable behaviours, not effort"
status: done
owner: "sys-opus-2026-09-12/implementer-check-boxes-lint-as-observable-behaviours"
level: 10
priority: P2
estimate: 2h
actual: committed 7cb9152, b450489 (the probe, rebased onto the trunk) and 9cd6268 on work/check-boxes-lint-as-observable-behaviours in .claude/worktrees/check-boxes-lint-as-observable-behaviours — memo-level advisory warning in `check_warnings` beside `validate`, attached to the write result, README row and rule, and the test extended to re-read the warned memo off disk; all four Check boxes ticked with the evidence above; the memo's `sh` block, `just check` and `just test` all green; lane left unlanded for the coordinator.
needs: ["[[the-work-can-be-proven]]"]
---

# check-boxes-lint-as-observable-behaviours

## Do

The analyst contract already says every Check box must be "a behaviour a
command can fail — never effort, never commit". Make it mechanical instead of
diligence: the memo cartridge's write validation warns (or fails under a
strict flag) when a `kind: work` memo carries `## Check` boxes that look like
effort statements — no command-shaped tokens (`just`, `cargo`, `python`,
`zirkle call`, backticked commands) anywhere in the box text or the memo's `sh`
block.

Scope: the validator in `builtin/memo/src/record.rs` (or a sibling check beside
the existing link validation), plus tests covering a good Spec, an effort-phrase
box, and a memo with no sh block.

## Check

- [x] Writing a `kind: work` memo whose `## Check` boxes name only effort —
      no backticked span in the section, no fenced `sh` block in the memo —
      saves the memo and returns a `warnings` entry quoting the first box and
      naming the memo path.
- [x] A memo whose box names its command, a memo whose boxes are exercised by
      a fenced `sh` block, a Check that only rolls up children
      (`[[child]] is done`), and a memo with no `## Check` at all each write
      with no `warnings` key.
- [x] The warning never refuses the write: `saved` is still `true` for the
      effort memo and the file is on disk.
- [x] `cargo nextest run -p memo_cartridge` is green (29 tests) and
      `just check` passes.

Observed, 2026-09-12, on the lane's build of `memo_cartridge`:

`a_work_check_that_names_no_command_is_saved_with_a_warning` covers the first
three boxes in one test. Forcing its path assertion red once printed the
literal warning the write returns:

    work/effort.md: ## Check names nothing to run — "Review the code and
    agree". A box is a behaviour a command can fail: name the command, or add
    an sh block that exercises the boxes.

The same test then asserts `saved["saved"] == true`, re-reads
`.zirkle/memos/work/effort.md` off disk and compares it byte for byte with what
was written, and walks the four clean shapes — `` `just check` passes ``, a
box plus a fenced `sh` block, `[[effort]] is done`, and a memo with no Check —
asserting `saved.get("warnings") == None` for each.

    PASS [   0.174s] (1/1) memo_cartridge ...
      a_work_check_that_names_no_command_is_saved_with_a_warning
    Summary [   4.630s] 29 tests run: 29 passed, 0 skipped
    just check: cargo fmt, clippy -D warnings, memory-check and the ui
      tsc gate all clean

```sh
cd /Users/feb/dev/sys/.claude/worktrees/check-boxes-lint-as-observable-behaviours
# The repo shares one cargo target dir across every worktree, so a sibling
# lane's build can hand this crate a stale `zirkle` rlib. Touch before running.
touch core/sdk.rs
cargo nextest run -p memo_cartridge -E 'test(a_work_check_that_names_no_command)'
cargo nextest run -p memo_cartridge
just check
```

## Approach

The probe on `work/check-boxes-lint-as-observable-behaviours` built the whole
thing; commits `d198d97` and `7f6f807`. What is left is running the gates and
landing.

Files touched:

- `builtin/memo/src/record.rs` — new `check_warnings(&Memo) -> Vec<String>`
  above `validate`, and three lines in the `op == "write"` arm of `execute`
  that attach a `warnings` array to the result when it is non-empty.
- `builtin/memo/tests/tests.rs` — `a_work_check_that_names_no_command_is_saved_with_a_warning`.
- `builtin/memo/README.md` — the `write` row gains `may return warnings`, and
  a paragraph states the rule and the rollup exemption.

What the probe established, and why the shape is what it is:

- **Warn, never refuse.** Measured against the live record: of the 49 work
  memos carrying `## Check`, 17 have no backticked span in the section and no
  `sh` block, and 83 of 182 individual boxes have no backtick. A hard failure
  would make a third of the record unwritable, and a per-box rule would be
  45% false positives on boxes that are perfectly good observable behaviours
  ("a test resizes the emulator; rows rewrap and the cursor stays on its
  line"). So the lint is memo-level and advisory. The strict flag the Do
  offers as an alternative is not built — nothing consumes it yet, and the
  record has to be cleaned before a gate can hold.
- **Rollup exemption.** When every box contains a `[[link]]`, the Check is a
  parent rolling up children (`[[tool-graph-engine…]] is done`); the commands
  live in the children's own boxes, so those memos are skipped.
- **No wiring needed.** `service.rs` stringifies the whole `record::execute`
  result into the tool's `content`, so the new field reaches the agent with no
  service change. Nothing else in the tree asserts the write result's shape
  (checked: no other consumer reads `saved`).

Steps for the hand, in order:

1. `just lane check-boxes-lint-as-observable-behaviours`; the probe is already
   on the branch.
2. Run the `sh` block above and tick the boxes.
3. `just test` for the full suite, then land.

Trap the probe hit twice: `.cargo/config.toml` points every worktree at one
shared `target/`, and a concurrent build in a sibling lane leaves a stale
`zirkle` rlib, so `cargo nextest run -p memo_cartridge` fails with
"no method named `inventory` found for `&zirkle::sdk::Host`". `touch core/sdk.rs`
clears it. This is not a regression in this work.

Derived from the unlazy analysis, 2026-09-12.
