---
kind: routine
description: Summarize a file or diff for the user. Use when the user asks "what does this do", "summarize",
  or "explain this file".
uses:
- usage: '[[run-usage]]'
  when:
  - Summarize a file or diff for the user. Use when the user asks "what does this do", "summarize", or
    "explain this file".
  tags:
  - summarize
  - explain
  - agent
  - docs
---

## Inputs

No prerequisites beyond the commands named below. Recipe parameters are named in Do; supply paths and arguments for the current task.

## Do

**Brief.** Read the file at `path` and produce a tight summary: one sentence on
what it is, a short bulleted list of what it does, one line on the most likely
reason a reader would open it. Skip boilerplate. If `path` is a directory,
summarize its purpose and list notable files. Do not speculate beyond the file.

```just
summarize path:
  text=`cat {{path}}`
  claude --print --prompt "Summarize this file for a developer. $(text)"
```

`path` is required (no default).

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `summarize` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
