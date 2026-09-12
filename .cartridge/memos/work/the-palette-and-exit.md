---
kind: work
description: "The UI lists everything it can call behind a fuzzy palette, and exits on command"
status: blocked
owner: "sys-opus-2026-09-12/implementer-the-palette-and-exit"
level: 10
priority: P1
estimate: 4h
---

# the-palette-and-exit

## Do

The terminal wrapper has no way out and no way to discover what it can do. Two
gaps, one surface:

1. A palette over every callable action: the bound keys (composer, transcript,
   paste, ...), the agent tools the profile exposes (tool.shell, tool.memo,
   ...), and the UI's own view commands. One key (Ctrl+P, unless taken) opens
   it; typing filters fuzzily; Enter runs the selected action and Escape
   closes without acting. The list is built from what is actually enabled —
   guards decide membership, nothing is hardcoded twice.
2. An exit: the palette names it ("exit") and a direct key (Ctrl+Q, unless
   taken) does the same — quit the wrapper cleanly, shell and sessions left
   as the user closed them.

Follows [[the-agent-surface-preserves-the-visible-shell]]: the palette covers
rows, it never replaces the shell.

Recorded from the user's request, 2026-09-12: "we still need a way to exit and
see all commands we can call with fuzzy searching".


## Direction from the user, 2026-09-12

The idea of the tool is that we never see the agent. We are just wrapping the
terminal; pressing the shortcut dispatches a job to the agent, it runs, and the
surface goes back to being just a terminal. The palette is the dispatch
surface: pick the command or job, the agent works invisibly, its results land
where the shell already shows things (scrollback / transcript on demand), and
no agent chrome stays on screen afterwards.


## Direction from the user, 2026-09-12 (second)

Think of it as a chat at the fingertips of the terminal — the /btw command from
Claude Code, but in a shell. A shortcut puts you in chat mode: what you type is
dispatched as a job to the agent, it runs, and the terminal is just a terminal
again. When needed, the transcript can be focused and forked into a
conversation.


## Approach

The probe built the whole thing and it works. It stands on the lane worktree
`/Users/feb/dev/sys/.claude/worktrees/the-palette-and-exit`, branch
`work/the-palette-and-exit`, commits `84ac1be`, `bcb2291`, `57a66ed`. What is
left is documentation, the full gates and landing.

**Files the probe touches**

- `builtin/ui/src/registry.ts` — a fourth contribution kind, `action`:
  `ui.action(name, label, run)` inside `setup`, and `actions()` returning every
  installed owner's actions in name order, first claim of a name winning.
  Membership is the existing feature guard: an owner's actions arrive with its
  install and vanish with `remove`, so nothing is enabled twice.
- `builtin/ui/src/index.ts` — exports `ActionEntry`.
- `builtin/ui/ui/palette.tsx` (new) — `fuzzy` (subsequence score, word starts
  and consecutive runs preferred), `paletteEntries` (matches best first, and a
  nonempty query always also offers itself as an agent job), and
  `registerPalette(ui, chat)` which registers the `shell.palette` slot: the
  Ctrl+P / Ctrl+Q key handling, the overlay box at zIndex 30, the filter
  textarea, and the built-in entries (agent tools from `chat op=tools`, paste,
  transcript, composer, exit).
- `builtin/ui/ui/terminal.tsx` — calls `registerPalette`, renders
  `<Slot name="shell.palette" />` at the surface root so it overlays every
  mode, and releases the embedded terminal's and the composer's focus while
  `chat.palette()` is true.
- `builtin/ui/src/chat.ts`, `builtin/ui/src/main.tsx`, `.zirkle/default/init.lua`
  — `chat` answers `op: "tools"` with the profile's tool keys, fed from the one
  `tools` table the profile already builds for the agent entry.
- `builtin/ui/tests/palette.test.tsx` (new), `tests/fixtures/shell/ui/terminal.tsx`,
  `tests/fixtures/counter/ui/terminal.tsx` — the shell fixture hosts the shipped
  palette with a stub chat, the counter fixture contributes one guarded action.
- `core/tests/test_ui.py` — the palette over the fixture surface, and Ctrl+Q.
  The fixtures are now composed at their repo-relative paths, because the shell
  fixture imports the shipped `ui/palette`.
- `core/tests/test_shell.py` — one new test over the real PTY surface, plus
  `self.original`, the line discipline read right after `pty.fork` and before
  the child has taken the terminal raw.

**What the probe already did** — all of it, green, re-run on tip `57a66ed`:

- `just check` clean (fmt, workspace clippy `-D warnings`, `bun run check`).
- `bun run --cwd builtin/ui test`: 23 pass, 0 fail, including the new
  `palette.test.tsx`.
- `python3 -m unittest discover -s core/tests -p test_ui.py`: 1 test, OK.
- `python3 -m unittest core.tests.test_shell.WrappedShell.test_the_palette_dispatches_from_either_mode_and_ctrl_q_exits`
  passes, three runs in a row (7-12s each); the whole `test_shell.py` file,
  7 tests, OK in 31s.
- `cargo nextest run --workspace`: 223 run, 223 passed. `cargo test --doc`,
  `builtin/tools/tests` (7), `test_development.py`, `test_router.py` (13) all
  green. `memory-test` green.
- Two flakes seen once each and not reproducible alone — both known contention
  timeouts from parallel lane gates on the shared target, neither touching this
  work: `zirkle tests::process::ordinary_lua_composition_can_load_a_process_wrapper`
  (`Elapsed(())`; passes alone in 1.8s) and
  `kern::e2e retention::queued_ingestion_preserves_the_submitted_expiration`
  (during an ENOSPC window on the shared `target/`).

Three things the probe had to find out, so they are not re-derived:

1. The palette's textarea swallowed Enter. It needs `defaultTextareaKeyBindings`
   with `up`/`down`/`return` filtered out and `return`/`kpenter` rebound to
   `submit`, and `run()` guarded by `if (!chat.palette()) return` because Enter
   reaches both the textarea's `onSubmit` and the global key handler.
2. The PTY tests accumulate output in one buffer, so an *absence* assertion is
   only meaningful after `self.screen.clear()` and a fresh redraw.
3. Cursor-optimised redraws interleave neighbouring cells, so those tests match
   short unique tokens (`exitzirkle`, `approvebash`), never whole phrases. In
   `test_ui.py` the palette overlays the shell transparently, hence the `see()`
   helper matching within one screen row with gaps allowed.

**What is left**

1. `cd /Users/feb/dev/sys/.claude/worktrees/the-palette-and-exit` — the lane
   exists and holds the probe; continue it, do not re-create it.
2. Document the surface, which the probe did not: in `builtin/ui/README.md`,
   `ui.action(name, label, run)` and `actions()` under "Composition and
   overrides", the `shell.palette` slot in the registered-slot list, the Ctrl+P
   and Ctrl+Q keys and the job-dispatch behaviour in the prose, and the
   `tools = tools` profile entry in the Lua snippet. In `CLAUDE.md`, add Ctrl+P
   and Ctrl+Q to the sentence that already names Ctrl+G and Ctrl+F.
3. Rebase onto the trunk tip and re-run `just check` and `just test`. Both were
   green on `57a66ed`; the gates race the shared `target/`, so a lone `Elapsed`
   timeout is re-run alone before it is believed, and an ENOSPC there means the
   shared target needs clearing or an isolated
   `CARGO_TARGET_DIR=/Users/feb/dev/sys/target/palette-gate`.
4. Run the `sh` block below, tick the boxes, then land.

## Observed, 2026-09-12 (implementer)

Blocked on one thing only, outside this memo: `builtin/memory/cartridge.json`
(gitignored, symlinked from every lane into the trunk's copy) now names
`"binary": "memory_cartridge"`, a binary no recipe builds — `just memory-build`
still builds `--bin memory` — so `test_router.py`'s three `LaunchedAgent` tests
fail in `just test`, in this lane and in the trunk checkout alike. When that
migration finishes, re-run `just test` and the last box closes.

The lane is rebased onto the trunk after the `momo` → `zirkle` rename (`c7a0e0d`)
and carries the palette work expressed in the new names: the profile comment and
`config.tools` entry in `.zirkle/default/init.lua`, `@zirkle/ui` imports, the action
label `exit zirkle · Ctrl+Q`, and the tests that assert it (`exitzirkle` in
`test_shell.py`, the cross-word fuzzy probe `exci` in `palette.test.tsx`).

Five boxes are closed on the current binary, under
`CARGO_TARGET_DIR=/Users/feb/dev/sys/target/palette-gate` (an isolated target,
because sibling lanes rewrite binaries in the shared one mid-run; that directory
was deleted afterwards):

- `bun --conditions=browser test tests/palette.test.tsx`: `3 pass, 0 fail`; the
  whole suite `23 pass, 0 fail, Ran 23 tests across 10 files`.
- `python3 -m unittest discover -s core/tests -p test_ui.py`: `Ran 1 test in
  7.379s / OK`.
- `python3 -m unittest core.tests.test_shell.WrappedShell.test_the_palette_dispatches_from_either_mode_and_ctrl_q_exits`:
  `Ran 1 test in 6.697s / OK`; the whole file `Ran 7 tests in 23.411s / OK`.
  The one test carries both the palette and the exit box: the wrapper ends on
  Ctrl+Q with status 0 and the line discipline is the one read before it started.
- `builtin/ui/README.md` documents `ui.action`/`ui.actions()`, the
  `shell.palette` slot and both keys; `CLAUDE.md` names Ctrl+P and Ctrl+Q beside
  Ctrl+G and Ctrl+F. All four greps of the `sh` block print their match.

`just check` is `exit 0` on this tip. `just test` is green everywhere the palette
reaches — `memory-test` `1331 tests run: 1331 passed, 17 skipped`,
`cargo nextest run --workspace` `249 tests run: 249 passed, 1 skipped`,
`cargo test --doc --workspace` ok, `builtin/tools/tests` `Ran 7 tests / OK`,
`zirkle --profile tools verify` `1 contracts passed`, `test_development.py` OK —
but it stops in `test_router.py`, whose three `LaunchedAgent` tests launch the
`memory` cartridge. `builtin/memory` is gitignored and symlinked from every lane
into the trunk's own copy, which is being rewired right now: its `cartridge.json`
changed under the run from `"binary": "memory"` to `"binary": "memory_cartridge"`,
and the trio failed in turn with the file missing
(`builtin/memory/cartridge.json: No such file or directory`) and with
`error: unrecognized subcommand 'hello'` from the binary that manifest still
names. The same three fail identically in the trunk checkout itself
(`Ran 3 tests in 32.474s / FAILED (failures=3)`), so this is not the lane.

Flakes seen and dismissed, each green alone on the same binary and each racing
another lane's concurrent gate: `retention::queued_ingestion_preserves_the_submitted_expiration`,
`agent::loop harness_system_inserted_once_...` and
`scripted_tool_call_then_final_answer_...` (both `` `agent` is not provided``),
and the `vanished_root_reaper` trio.

## Check

- [x] `bun run --cwd builtin/ui test` passes `palette.test.tsx`: an installed
      owner's action appears in the palette and disappears when that owner is
      removed, a fuzzy query narrows the list to one entry, Enter runs it and
      closes, a query matching no action dispatches as an agent job, and Escape
      closes without running anything.
- [x] `python3 -m unittest discover -s core/tests -p test_ui.py` passes: Ctrl+P
      over the composed fixture surface lists `increment counter` and
      `exit zirkle`, typing `incr` narrows to the one action, Enter increments
      through the Lua service, and after the counter cartridge is removed from
      the profile its action is gone from the palette while `exit zirkle` remains.
- [x] `python3 -m unittest core.tests.test_shell.WrappedShell.test_the_palette_dispatches_from_either_mode_and_ctrl_q_exits`
      passes: Ctrl+P opens the palette over the real shell and again over the
      composer in agent mode, Escape closes it without acting and the shell
      still answers, and a query matching no action dispatches as a job that
      reaches the agent.
- [x] The same test proves the exit: Ctrl+Q ends the wrapper with exit status 0,
      and the terminal's `ICANON|ECHO` afterwards is the value read before the
      wrapper started, not the raw mode it ran in.
- [x] `builtin/ui/README.md` documents `ui.action`, the `shell.palette` slot and
      the Ctrl+P / Ctrl+Q keys, and `CLAUDE.md` names both keys beside Ctrl+G
      and Ctrl+F.
- [ ] `just check` and `just test` are green on the lane.

```sh
cd /Users/feb/dev/sys/.claude/worktrees/the-palette-and-exit
bun install --cwd builtin/ui --frozen-lockfile
bun run --cwd builtin/ui check
bun run --cwd builtin/ui test
cargo build -p zirkle -p sessions --bins
python3 -m unittest discover -s core/tests -p test_ui.py
python3 -m unittest core.tests.test_shell.WrappedShell.test_the_palette_dispatches_from_either_mode_and_ctrl_q_exits
grep -q 'ui.action' builtin/ui/README.md
grep -q 'shell.palette' builtin/ui/README.md
grep -q 'Ctrl+P' builtin/ui/README.md && grep -q 'Ctrl+Q' builtin/ui/README.md
grep -q 'Ctrl+P' CLAUDE.md && grep -q 'Ctrl+Q' CLAUDE.md
just check && just test
```

## Landed

On the trunk as `6ce8591` (lane `work/the-palette-and-exit`, rebased onto
`87d4d85`, 16 files). Landed at the user's direction with the sixth box still
open: the three `test_router.py::LaunchedAgent` failures come from the
in-flight `builtin/memory` rewiring — `cartridge.json` names
`"binary": "memory_cartridge"` and no recipe builds it — and they fail
identically in the trunk checkout, so this lane neither causes them nor makes
the trunk worse. `bun run --cwd builtin/ui test` re-run on the trunk after the
merge: `23 pass, 0 fail`.

Box six closes on one green `just test` once the memory repo is fixed. Until
then this memo stays `blocked` rather than `done`, because the box was never
observed.
