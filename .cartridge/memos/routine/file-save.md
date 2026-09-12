---
kind: routine
description: Write content to a file path, creating parent directories and overwriting any existing file.
  Use when the agent needs to write a whole file (new file, or full replacement) rather than an in-place
  regex edit. Takes the path and the content as arguments.
uses:
- usage: '[[run-usage]]'
  when:
  - write a file
  - save content to a path
  - create a new file
  - overwrite a file wholesale
  - write these bytes to disk
  not_when:
  - a small in-place find-and-replace
  - open a file in an editor
  - read a file
  tags:
  - file
  - save
  - write
  - create
  - overwrite
---

## Inputs

Requires on PATH: `sh`. Recipe parameters are named in Do; supply paths and arguments for the current task.
Risk: side effects file-write; danger high.

## Do

Write a whole file to a path. Creates the parent directory if missing and
overwrites the target if it exists — this is a wholesale write, not a merge, so
it is `danger: high`. For a surgical in-place substitution prefer [[text-sed]]
(`inplace`); to open a file in an editor see [[vim-edit]]. The content arrives as a
positional argument alongside the path, so a single tool call carries both.

| Recipe | Does |
|--------|------|
| `save` | write `content` to `path` (mkdir -p the parent, overwrite) |

```just
# write content to path, creating parent dirs and overwriting (default)
save path content:
  #!/usr/bin/env sh
  set -eu
  p={{quote(path)}}
  mkdir -p "$(dirname "$p")"
  printf '%s' {{quote(content)}} > "$p"
  echo "wrote $p"
```

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `save` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
