---
kind: routine
description: Apply the same headless vim edit across every file matching a glob, in place. Use for a project-wide
  substitute, deletion, or ex script with no UI.
uses:
- usage: '[[run-usage]]'
  when:
  - edit many files at once
  - project-wide substitute
  - run ex over a tree
  - bulk vim edit
  tags:
  - vim
  - batch
  - edit
  - automation
  - headless
---

## Inputs

Requires on PATH: `nvim`. Recipe parameters are named in Do; supply paths and arguments for the current task.
Risk: side effects file-write; danger medium.

## Do

Fans one set of ex commands across every matching file, in place — each file
edited the way [[vim-edit]] edits one. The glob is matched under the current tree;
the commands are ordinary ex (see [[vim-ex]]) over vim-regex patterns ([[vim-search]]).

```just
# apply ex commands to every file matching a glob, in place
batch glob cmds:
  #!/usr/bin/env sh
  set -eu
  cmds={{quote(cmds)}}
  find . -type f -name {{quote(glob)}} -print0 \
    | xargs -0 -I{} nvim --headless -c "$cmds" -c 'wq' -- {}
```

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `batch` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
