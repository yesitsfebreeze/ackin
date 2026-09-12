---
kind: note
description: Cartridge is the active development home; sys is a temporary legacy session link
---

> We want to continue developing here

Develop from `~/dev/cartridge/cartridge.ctg`. The parent Justfile forwards to
source commands. Each loadable cartridge has `cartridge.json` with its test,
build and process commands. Use `just test <module>` and `just describe <module>`.

The sys checkout was moved to `.cartridge/legacy-sys` with its Git history and
worktrees preserved. `~/dev/sys` links there only for still-running sessions.
Private state was moved into the new repositories with backwards links; the
active Markdown record is a migration snapshot. Do not delete the legacy copy
before checking for later session writes and closing its processes.

> The cartridge JSON should already describe its behavior and the memos should do that as well

`cartridge.json` is the single cartridge document: behavior, dependencies, grants,
purpose, and runnable command arguments. There is no separate development JSON.
Memos explain development context and decisions; the parent Justfile selects
modules. For a memory crate, use `just test memory/<Cargo package name>`, for
example `just test memory/store`. Runtime and library commands are in their
README files because those repositories are not loadable cartridges.
