---
kind: work
description: "The pty cartridge runs a terminal emulator over the shell and serves its grid, viewport and mark rows"
status: done
owner: "sys-opus-2026-09-12/implementer-pty-owns-the-terminal-grid"
actual: 3h

---

## Spec

Probe committed on `work/pty-owns-the-terminal-grid`: `513a340` puts the
emulator in `builtin/pty/grid.rs` — a new `Emulator` over
`alacritty_terminal` 0.26 providing grid, scrollback, viewport, alt-screen and
cursor, damage frames, and absolute row attribution for command blocks.
`main.rs` serves the ops `screen`, `viewport`, `scroll` and `frame`; `commands`
carries `row`; config gains a `scrollback` key; `tool.rs` and `marks.rs`
(`pub fn osc`) updated; `Cargo.toml` swaps `vt100` for `alacritty_terminal`.

Probe status: `just check` green; `cargo test -p pty` fully green (8 unit + 6
process, including the real nvim screen test). The full-suite failures in its
log (`/tmp/pty-grid-gate2.log`) were proven contention flakes in the shared
target — the profile and memory-kern tests pass in isolation; treat them as
environmental and gate isolated.

Remaining for the implementer:

1. Re-read the probe diff and this Check; finish anything the probe left
   half-wired (frame versioning from a caller-supplied version number, scroll
   op bounds).
2. Rebase the lane onto trunk; run `just check` and `cargo test -p pty`
   isolated (`CARGO_TARGET_DIR=/Users/feb/dev/sys/target/pty-grid-gate`).
3. Tick the Check boxes one at a time with quoted output, then run the sh
   block.
4. `just all` on the rebased lane as the final gate.

Estimate: 4h remaining.

Implemented. The lane is rebased onto the trunk and carries three commits:
`6f87598` (the probe, replayed), `65bb945` bounding `frame` and `scroll`, and
`55c8c5e` proving the one-step resize over the real RPC path. `frame` now
remembers the version its held damage was reset at, so a caller further back
than that is given the whole viewport instead of rows that do not describe its
gap; `scroll` clamps the delta to the grid's extent, which the grid's own
`i32` addition would otherwise overflow, and the `scroll` op reads the
viewport under the lock it scrolled.

The gates must run with their own target directory, as the Spec says: the
shared `target/` is written by several worktrees at once and left a stale
`zirkle` rlib against which `memo_cartridge` does not compile
(`no method named 'inventory' found for reference '&zirkle::sdk::Host'`, a
method the lane's own `core/sdk.rs:50` plainly has), and `just memory-test`
fails on the trunk too, on a different `kern::e2e` test each run. A fresh
isolated target also needs `cargo build --workspace --bins` once before
`just test`, or `core/tests/test_router.py` cannot find `proxy`.


# pty-owns-the-terminal-grid

## Outcome

Every byte the shell writes passes through an emulator inside `pty`: a grid of
styled cells, a scrollback of configurable depth, a viewport offset, an
alternate-screen flag and a cursor. The marks parser records the absolute row at
which each command's prompt, command line and output began, so every scrollback
row can name the command block it belongs to, and `commands` carries that row.

`pty` serves the grid to any caller: `screen` returns the rows of a viewport as
styled cells with default colours left as "default", never as white on black;
`viewport` returns offset, total rows, cursor and alternate-screen state; `scroll`
moves the viewport; `frame` returns the rows changed since a version number, so a
painter draws only damage. The output ring and `read` stay until
[[the-embedded-terminal-is-gone]] removes them.

## Check

- [x] A Rust test feeds two commands with marks through the emulator and reads
      back each command's row; after enough output to scroll, the rows still
      point at the right lines in scrollback.
- [x] A test enters the alternate screen, draws, leaves it, and finds the
      primary screen and scrollback untouched, with `viewport` reporting the
      switch while it lasted.
- [x] A test resizes the emulator and the shell in one step; rows rewrap and the
      cursor stays on its line.
- [x] `frame` after a version with no change returns no rows; after one write it
      returns only the rows that write touched.

```sh
CARGO_TARGET_DIR=/Users/feb/dev/sys/target/pty-grid-gate cargo test -p pty
```

Observed on the lane at `55c8c5e`, isolated target
`/Users/feb/dev/sys/target/pty-grid-gate`:

- Box 1 — `cargo test -q -p pty --bins -- --exact grid::tests::rows_survive_scrolling`
  → `test result: ok. 1 passed; 0 failed`; same for
  `grid::tests::rows_survive_a_full_scrollback`, which reads the surviving
  command's rows back after the scrollback has trimmed.
- Box 2 — `... --exact grid::tests::alternate_screen_leaves_primary_and_scrollback`
  → `test result: ok. 1 passed; 0 failed`.
- Box 3 — `... --exact grid::tests::resize_rewraps_and_keeps_cursor_line`
  → `test result: ok. 1 passed; 0 failed`, and over the real RPC path
  `cargo test -p pty --test process resize_moves` →
  `test resize_moves_the_emulator_and_the_shell_together ... ok`: one `resize`
  call, `stty size` in the wrapped shell reports `30 100` and `viewport`
  reports 30 rows with 100 cells a row.
- Box 4 — `... --exact grid::tests::frame_reports_damage_since_version`
  → `test result: ok. 1 passed; 0 failed`.
- The `sh` block — `cargo test -p pty` →
  `test result: ok. 9 passed` (unit) and `test result: ok. 7 passed` (process).
- Whole workspace in the isolated target —
  `cargo nextest run --workspace --no-fail-fast` →
  `Summary [40.662s] 236 tests run: 236 passed, 0 skipped`.

Final gate, `CARGO_TARGET_DIR=/Users/feb/dev/sys/target/pty-grid-gate` after one
`cargo build --workspace --bins`:

- `just check` → exit 0.
- `just test` → exit 0;
  `Summary [56.620s] 1337 tests run: 1337 passed, 17 skipped` (memory),
  `Ran 20 tests across 9 files` (ui),
  `Summary [35.600s] 236 tests run: 236 passed, 0 skipped` (workspace), and
  every python suite `OK`.

## Approach

Use an existing crate for the emulator; `alacritty_terminal` and `wezterm-term`
both carry scrollback, damage tracking and a display offset. Pick by which one
feeds bytes without owning the PTY, since `pty` already does. One emulator per
shell, behind the same lock as the marks. Row attribution is a `Vec` of block
ids parallel to the scrollback, trimmed as the scrollback trims.
