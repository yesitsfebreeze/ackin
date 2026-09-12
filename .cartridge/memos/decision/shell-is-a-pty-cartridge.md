---
kind: decision
description: The wrapped shell is owned by the pty cartridge, never by the UI process; the UI only draws it
status: superseded
date: "2026-09-11"
decided_by: user
---

## Superseded on 2026-09-12

Follow [[the-terminal-grid-lives-in-pty]] and its successor for the current contract. The original
choice and rationale below are historical; they are not instructions to remove
the latest-tool footer, status widgets or full transcript. PTY ownership and
UI-independent shell lifetime remain required.

## Choice

zirkle is a terminal wrapper with an inline agent, after
[bombshell](https://github.com/yesitsfebreeze/bombshell), built on the cartridge
system. The `pty` cartridge spawns the shell in one PTY at apply and keeps it for
its own lifetime; it serves `write`, `resize`, `print`, `read` and publishes
output chunks as `pty` events. The `ui` cartridge draws that stream in an
embedded terminal, adds the status bar and the Ctrl+G composer, and prints agent
replies into the same stream through `print`. It never spawns the shell itself.

## Why

The shell must outlive UI replacement and reloads, and the agent runs beside the
terminal rather than inside the UI process. A shell that is a child of the Bun
process dies with every UI swap and ties the agent's output path to one
renderer.

## Consequences

Any cartridge can print into the terminal through `pty.print`; the UI is one
consumer of the stream. Terminal emulation stays in the UI (OpenTUI's embedded
terminal); reattachment replays the cartridge's recent output ring.

Those two sentences no longer hold. [[the-terminal-grid-lives-in-pty]] moves the
emulator into this cartridge, which makes the output ring and `print` unnecessary
and retires both. Everything else here stands: the cartridge still owns the PTY
for its own lifetime, and the UI still never spawns the shell. The `pty`
cartridge also serves `environment`, the boot-time scan of shell, `PATH` and
executables, for anything that needs to know what the host offers.
