---
kind: grammar
description: "the user's wrapped shell and the tool that drives it — not the removed shell cartridge"
overloads: the wrapped shell, tool.shell, shell integration, the removed shell cartridge
date: "2026-09-12"
---

# shell

Four senses. The **wrapped shell** is the user's own shell, spawned once by
the pty cartridge at apply and kept across UI replacements — nu by default.
**`tool.shell`** is the agent tool over it: submit commands, send interactive
input, read the screen. **Shell integration** is the OSC 133/633/7 marks the
pty cartridge parses for nu, zsh and bash so commands carry cwd, exit and
output. And **the shell cartridge** (`builtin/shell`) is gone — removed on
the terminal-integration branch; its memos and the UI `command.ts` went with
it. Authority is `builtin/pty` and [[terminal-grammar]].

The bite: "shell" never means a cartridge-provided shell — cartridge wraps the user's,
it owns none. And a shell command is not the agent tool: the tool is
`tool.shell`, the shell is the user's.