---
kind: work
description: "Every part is a cartridge that can be added, replaced or removed while the process runs"
status: done
subwork:
  - "[[reload-test-asserts-the-shell-first-surface]]"
uses:
  - usage: "[[read-usage]]"
    when: ["Changing the loader, the profile, hot reload, UI registration or the guards that enable features"]
---

# cartridges-compose-live

## Outcome

Nothing in zirkle is welded in. A profile lists path entries; each is
independently replaceable by editing its manifest, Lua entry or profile entry,
without rebuilding the host. Edited sources rebuild and the changed cartridge is
replaced in place while the session runs. A cartridge that is not enabled leaves
no trace: no services, no UI contributions, no record.

Scope is the loader, the profile, the reload path, and the UI registry's slots
and guards. What any individual cartridge does belongs to its own memo.

## Check

- [x] `core/tests/test_development.py` edits a cartridge's wrapper under a live
      host and waits for a second alternate-screen entry, then for the surface
      to come back, with the host process still alive.
- [x] The same test writes invalid Rust, waits for `development: build failed;
      current cartridges retained`, and asserts the process is still running.
- [x] `core/tests/test_ui.py` drops `theme` from the profile and waits for
      "custom surface" to leave the screen while the host stays up, then drops
      `counter` and waits for its contributions to go too.
- [x] Enabled: `system/resolver-guidance.md` ships only in
      `builtin/memo/.zirkle/memos/` and appears in the composed system list.
      Absent: with only `memo` and `clock` enabled, the record lists 16 notes and
      none of `builtin/ui`'s two dozen `tui-*` notes.

## Result

Nothing here needed building; it needed proving, and the proof was blocked by
[[reload-test-asserts-the-shell-first-surface]] until that landed. Hot
replacement, retention on a failed build, live removal and record isolation are
each covered by a test that fails if the property breaks.

The one thing no test covers is a cartridge whose *manifest* is edited rather
than its source. If that path breaks, it will break quietly.
