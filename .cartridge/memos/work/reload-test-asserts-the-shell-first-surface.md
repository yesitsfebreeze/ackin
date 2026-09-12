---
kind: work
description: "The development reload test asserts the shell-first surface, so just test is green on the trunk"
status: done
uses:
  - usage: "[[read-usage]]"
    when: ["just test fails in core/tests/test_development.py, or the reload test's assertions no longer match the surface"]
---

# reload-test-asserts-the-shell-first-surface

## Outcome

`just test` is green on the trunk. `core/tests/test_development.py` was written
against a surface where the composer was always visible. Since the shell wrapper
landed in `45a6774` the startup screen is the wrapped shell, the composer is
behind Ctrl+G, and Ctrl+C belongs to the shell rather than to the wrapper — so
both the test's idea of what is on screen and its way of shutting down are
stale.

Its subject is still right and worth keeping: a cartridge replacement must leave
the surface alive, and a failed build must retain the current cartridges.

## Approach

The lane `work/reload-test` holds the first pass as `3d5dc63`: the two
`wait_for(lambda: "sends" in visible())` calls at lines 84 and 92 now wait for
`"SHELL"`, the status bar's mode chip, which the shell-first surface draws at
startup. With that change the run reaches line 98; every wait before it passes.

What is left is the shutdown. Line 97 sends `\x03`, but the embedded terminal is
focused in shell mode and its `onData` forwards every byte to the wrapped shell
(`builtin/ui/ui/terminal.tsx`, `shell.terminal`), so Ctrl+C never reaches the
renderer's `exitOnCtrlC`. `zirkle run` also ignores SIGINT on purpose
(`core/main.rs`: "Foreground process cartridges receive terminal interrupts
directly"), so signalling the host is not the answer either.

The real quit path is exiting the shell: `pty` publishes the exit, `alive()`
flips, `terminal.tsx:188` destroys the renderer, and `main.tsx`'s `onDestroy`
stops the host, which is what prints `development: stopped` and exits zero.

So:

1. Add `pty` to the test's fixture as `core/tests/profile.rs` already does: a
   `{id="pty",path=<ROOT/builtin/pty>}` entry, and `"pty"` in the `ui` entry's
   `inject`.
2. Replace the `os.write(master, b"\x03")` shutdown with input that exits the
   wrapped shell.
3. Run the test, then [[repository-checks]].

Two things the lane exposed that are not this memo's to fix, and want their own
memos if they bite again: the test resolves its binary at `ROOT/target`, which
is wrong in a lane where `.cargo/config.toml` redirects cargo to the trunk's
shared target; and it symlinks `builtin/ui/node_modules`, which a fresh lane
does not have until `bun install --cwd builtin/ui` runs.

## Check

- [x] `just all` passes in the lane, first run: 211/211 Rust tests, 11 bun
      tests across 4 files, 6 tools tests OK, 2 core terminal tests OK.
- [x] The first wait asserts `"SHELL"`, the status bar's mode chip, which the
      shell-first surface draws at startup.
- [x] The test exits the wrapped shell with `exit\r` and still asserts
      `os.waitstatus_to_exitcode(code) == 0`.
- [x] Both reload assertions are untouched: the alternate-screen count after an
      unrelated rebuild, and `development: build failed; current cartridges
      retained` with the process still alive.

## Result

Landed as `06a7d6c` and `ef3581b`. The waits assert the status bar's SHELL chip
instead of composer text, and the fixture gained the `pty` cartridge so the test
shuts the host down the way a person does — by exiting the shell. The run went
from timing out at 60s to passing in under 10.

The two lane problems this exposed were fixed first, under
[[the-work-can-be-proven]]: [[gates-run-in-a-lane]].
