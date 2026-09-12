---
kind: routine
description: Open a file, folder, or URL in the host's default application. Use when the user asks to
  open something in the GUI, reveal a path, or launch a link in the browser.
uses:
- usage: '[[run-usage]]'
  when:
  - open this file
  - open in the browser
  - launch the default app
  - reveal in finder
  - open a url
  not_when:
  - edit in the terminal
  - open in vim
  - read the file contents
  tags:
  - open
  - xdg-open
  - launch
  - default-app
  - url
  - cross-platform
---

## Inputs

No prerequisites beyond the commands named below. Recipe parameters are named in Do; supply paths and arguments for the current task.
Risk: side effects gui; danger low.

## Do

Hand a path or URL to whatever the OS calls the default handler. One recipe, four
hosts — the launcher differs per platform, so each is a guarded variant and the
runner emits only the one matching the host (`just`'s `[unix]`/`[macos]`/`[windows]`/`[wsl]`
attribute convention). On WSL, `wslview` (from
`wslu`) bridges to the Windows shell so a Linux path or URL opens on the Windows
side.

```just
[unix]
open target:
  xdg-open {{target}}

[macos]
open target:
  open {{target}}

[windows]
open target:
  cmd /c start "" {{target}}

[wsl]
open target:
  wslview {{target}}
```

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `open` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
