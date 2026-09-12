---
kind: work
description: "The obsolete UI emulator is removed while shell readback and agent footer survive"
status: open
estimate: 1d
needs:
  - "[[the-ui-paints-the-grid]]"
---

# the-embedded-terminal-is-gone

## Outcome

After the painter is proven, remove the duplicate UI emulator and its obsolete
palette post-compose pass, byte pump and keystroke encoding. Preserve the
latest-tool footer, main/dynamic status and composer/full transcript required by
[[the-agent-surface-preserves-the-visible-shell]]. Removing bottom chrome is
explicitly outside this cleanup.

Remove a `pty` read/print/output-ring path only after tracing its consumers and
providing equivalent shell readback, command evidence and UI reattachment.
Agent text does not enter the shell stream. A transport cleanup must not remove
the bounded output needed by the `shell` tool or compacted agent context.

## Check

- [ ] No `EmbeddedTerminalRenderable` or duplicate escape-sequence parser remains in UI runtime source.
- [ ] `just check`, `just test` and `just ui-check` pass without deleting behavioral coverage to make them pass.
- [ ] Shell readback, real nvim, UI replacement and latest-tool/transcript behavior still pass against the painter.
- [ ] Main status, dynamic status and the blank-before-first-tool footer remain available.
- [ ] `builtin/ui/README.md` and `CLAUDE.md` describe the delivered painter and preserved surface; historical evidence stays attributed.

The checks, exercised:

```sh
set -euo pipefail
# Check 1: no UI emulator or own escape-sequence parser remains in runtime source
! grep -rn "EmbeddedTerminalRenderable\|carryTheme" builtin/ui/src builtin/ui/ui
# Check 5: docs describe the painter; pty owns the grid
grep -q "paints the shell's grid" CLAUDE.md
grep -q "grid.rs" builtin/ui/README.md
# Check 2: gates
cargo fmt --all -- --check
cargo clippy -p pty -p momo --all-targets -- -D warnings
bun run --cwd builtin/ui test && bun run --cwd builtin/ui check
just test
# Checks 3-4: behavioural gates live in core/tests/test_shell.py (real nvim,
# UI replacement, transcript, main/dynamic status) and the pty process tests.
```

## Approach

One proven consumer migration per removal. Update current documentation with the
behavioral change. Keep historical memos; an unscoped search demanding that
historical names disappear would erase evidence rather than prove cleanup.

## Spec

The probe on the `work/the-embedded-terminal-is-gone` lane proves the removal:
`pty` still owns the terminal after `print`, `read` and the output ring are
gone, and the UI still draws, reattaches and reports. What the probe did, in
order:

1. `builtin/pty/main.rs` — removed the `Ring`/`RING_BYTES`/`ring` field, the
   `{n,data}` pty event in `publish`, the `read` op and the `print` op; a
   cursor query (`ESC [ 6 n`) is now answered from the emulator's cursor
   (`answer_cpr`), as a terminal would.
2. `builtin/pty/tests/process.rs` — `boot()` no longer reads raw chunks to
   catch the cursor query; added the `CPR_SHELL` probe test
   (`a_cursor_query_is_answered_without_the_ui_reading_raw_output`). `cargo
   test -p pty` passes: 9 unit + 8 process tests.
3. `builtin/ui/src/term.ts` — dropped the chunk buffer and `read`/`print`
   passthrough; forwards only `write`/`resize`/`frame`/`viewport`/`event` and
   serves `status` (exit + newest finished command) from the `command`/`exit`
   pty events.
4. Pinned `builtin/ui/ui/painter.ts`, `painter.test.tsx`, `terminal.tsx` and
   `transcript.test.tsx` verbatim from `work/the-ui-paints-the-grid` so a fresh
   checkout of this lane builds; `bun run --cwd builtin/ui test` (25 pass) and
   `bun run --cwd builtin/ui check` (tsc clean).
5. `builtin/ui/README.md` and `CLAUDE.md` now describe the pty-owned grid and
   the painted surface (Check 5).

Files touched: `builtin/pty/main.rs`, `builtin/pty/tests/process.rs`,
`builtin/ui/src/term.ts`, `builtin/ui/ui/terminal.tsx`,
`builtin/ui/ui/painter.ts` (pinned), `builtin/ui/tests/painter.test.tsx`
(pinned), `builtin/ui/tests/transcript.test.tsx`, `builtin/ui/README.md`,
`CLAUDE.md`.

What is left for the implementer: run `just test` on the combined tree so the
Python suite (`core/tests/test_shell.py`) proves replacement, real-nvim,
transcript and status behaviour against the painter, then `just land`. Anything
the painter itself still owes belongs to `the-ui-paints-the-grid`, not this
memo.
