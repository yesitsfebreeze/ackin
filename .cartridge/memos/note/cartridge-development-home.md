---
kind: note
description: Cartridge is the active development home; sys is a temporary legacy session link
---

> We want to continue developing here

Develop from `~/dev/cartridge/cartridge.ctg`. The parent Justfile forwards to
source commands. Each sibling repository has `development.json` with its test,
build and process commands. Use `just test <module>` and `just describe <module>`.

The sys checkout was moved to `.cartridge/legacy-sys` with its Git history and
worktrees preserved. `~/dev/sys` links there only for still-running sessions.
Private state was moved into the new repositories with backwards links; the
active Markdown record is a migration snapshot. Do not delete the legacy copy
before checking for later session writes and closing its processes.
