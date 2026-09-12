---
kind: grammar
description: "the wrapped shell's screen surface, the placeholder, and the term backend — three terminal names, one screen"
overloads: the term UI service, the terminal placeholder, the terminal UI module, the terminal generically
date: "2026-09-12"
---

# terminal

Four senses. **`term`** is the UI service: the wrapped-shell backend the one
Bun process provides over `pty`. The **terminal** system placeholder is what
the harness renders from the pty cartridge's recent commands — cwd, exit
codes, output. The **terminal UI module** is `builtin/ui/ui/terminal.tsx`,
the embedded terminal that fills the cartridge surface. Generically, "the
terminal" is that whole screen. Authority is [[shell-grammar]] for the thing
being shown, `builtin/ui/README.md` for the services.

The bite: three names for surfaces of one screen. `term` is a service, the
placeholder is data about the shell, the module is code — none of them is a
second terminal. The agent reaches the shell through `tool.shell`, never
through `term`.