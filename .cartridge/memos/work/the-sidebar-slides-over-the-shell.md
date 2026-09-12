---
kind: work
description: "The grid painter preserves the composer, full transcript, latest-tool footer and extension slots"
status: active
owner: "sys-work-2026-09-12/analyst-the-sidebar-slides-over-the-shell"
estimate: 1d
needs:
  - "[[the-ui-paints-the-grid]]"
---

# the-sidebar-slides-over-the-shell

## Outcome

The grid painter integrates the existing agent views under
[[the-agent-surface-preserves-the-visible-shell]]. Ctrl+G opens the composer;
Ctrl+F in agent mode opens the full transcript; Escape from the transcript
returns to the editor without cancelling the run. The editor retains its size,
contents and cursor. Composer keystrokes never leak into its input.

The bottom area shows only the latest tool and its output, with no message
before a tool exists. Preserve main status, dynamic status slots and cleared
glyphs underneath the transparent composer. Transcript/panel slots remain
extensible without prescribing a left sidebar or replacing the footer.

## Check

- [ ] `bun run --cwd builtin/ui test` passes `tests/agentview.test.tsx`: with the
      real chat surface, the latest-tool area is blank before any call, the
      fixture cartridge's `status.counter` stays visible above the transparent
      composer, the covered shell glyphs are gone from the frame, and Ctrl+F in
      chat mode opens the transcript without writing a byte to the shell.
- [ ] `bun run --cwd builtin/ui test` passes `tests/transcript.test.tsx` (the
      painter swap keeps it green): nvim's grid, size and cursor are unchanged
      across Ctrl+G, Ctrl+F and Escape; the latest-tool footer paints only the
      last call; older calls stay in the full transcript.
- [ ] `python3 -m unittest discover -s core/tests -p test_shell.py` passes after
      the painter lands: the real nvim passage leaves the prompt intact, and
      the composer/transcript opens and closes without taking the shell.
- [ ] No key crossing in the wire tests: composer keystrokes never reach the
      shell and shell-mode keys never reach the composer (asserted in
      `tests/transcript.test.tsx` and `tests/agentview.test.tsx`).
- [ ] The counter fixture registers `status.counter` in
      `builtin/ui/tests/fixtures/counter/ui/terminal.tsx` with no edit to
      `builtin/ui/ui/terminal.tsx` — the registration path is the module
      manager's `loadPlugin`, not the UI cartridge.

```sh
cd /Users/feb/dev/sys/.claude/worktrees/the-sidebar-slides-over-the-shell
bun install --cwd builtin/ui --frozen-lockfile
bun run --cwd builtin/ui check
bun run --cwd builtin/ui test
# after the painter lands:
python3 -m unittest discover -s core/tests -p test_shell.py
cd /Users/feb/dev/sys            # the lane is where the gate runs: see justfile
just check
just test
```

## Approach

Depends on [[the-ui-paints-the-grid]]: its swap keeps every agent-view slot
(`chat.composer`, `chat.latest-tool`, `chat.transcript`, `shell.statusbar`,
`status.*`, `surface.default`) with the same fixed-footer, transparent-composer
and `clearOverlay` structure, so this memo's checks hold before and after it.

Files:

- `builtin/ui/tests/agentview.test.tsx` — new (probe committed in this lane).
  Loads the real chat surface and the counter fixture through `loadPlugin`,
  drives the embedded terminal, and asserts: latest-tool blank before any call;
  `status.counter` visible above the transparent composer in chat mode; covered
  shell glyphs erased from the frame; Ctrl+F in chat opens the transcript with
  zero `write` ops to the shell.
- `builtin/ui/tests/fixtures/counter/ui/terminal.tsx` — register
  `status.counter` (probe committed). This is the fixture-cartridge side of
  check 5: a second module adds a dynamic status widget and the slot registry
  renders it, no UI-cartridge edit.
- `builtin/ui/tests/transcript.test.tsx`, `builtin/ui/ui/terminal.tsx`,
  `builtin/ui/tests/painter.test.tsx` — owned by the painter; this memo just
  re-runs them against its own checks once the painter lands.

Steps:

1. Land [[the-ui-paints-the-grid]]. Its lane already updates
   `terminal.tsx` and `transcript.test.tsx` to the painter and keeps the
   surface structure this memo's checks describe.
2. Keep the two probe files from this lane as-is (they run green now against
   the embedded terminal and stay green against the painter, since they drive
   only the surface slots).
3. `bun install --cwd builtin/ui --frozen-lockfile`, then
   `bun run --cwd builtin/ui check` and `bun run --cwd builtin/ui test`
   (21 UI tests, `agentview` included).
4. After the painter lands, run the real nvim passage in
   `core/tests/test_shell.py` and the full `just check` / `just test` gates.

What the probe already did: the two files above, both passing. What is left:
landing the painter, then re-running the gates with it in place.

Keep the historical filename for incoming links. On 2026-09-12 its outcome was
amended from a sliding sidebar to the user's composer/full-transcript contract.
The existing overlays in `builtin/ui/ui/terminal.tsx` are the starting behavior;
verify them again after the painter swap. This work depends on the painter,
not on adding a gutter, because the views must work without indicators.
