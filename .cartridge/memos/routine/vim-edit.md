---
kind: routine
description: Edit a file in place with vim, headlessly — apply ex commands, a substitute, a deletion,
  or a script with no UI. Use to script vim edits non-interactively.
uses:
- usage: '[[run-usage]]'
  when:
  - edit a file with vim non-interactively
  - headless substitute
  - scripted vim edit
  - apply ex commands to a file
  tags:
  - vim
  - edit
  - headless
  - ex
  - automation
---

## Inputs

Requires on PATH: `nvim`. Recipe parameters are named in Do; supply paths and arguments for the current task.
Risk: side effects file-write; danger medium.

## Do

Drives vim with no UI to change a file and save. `edit` is the default; `sub`,
`delete`, `apply` are convenience entry points over it. Commands are ordinary ex
(see [[vim-ex]]); patterns are vim regex (see [[vim-search]]). Delegates to
`nvim --headless`; for stock Vim swap the prefix for `vim -es`.

```just
# apply ex commands to a file, then save (default)
edit file cmds:
  nvim --headless -c {{quote(cmds)}} -c 'wq' -- {{quote(file)}}

# substitute every match across the file (s/pat/rep/g)
sub file pat rep:
  nvim --headless -c {{quote("%s/" + pat + "/" + rep + "/g")}} -c 'wq' -- {{quote(file)}}

# delete every line matching a pattern (g/pat/d)
delete file pat:
  nvim --headless -c {{quote("g/" + pat + "/d")}} -c 'wq' -- {{quote(file)}}

# source an ex/vim script against the file, then save
apply file script:
  nvim --headless -S {{quote(script)}} -c 'wq' -- {{quote(file)}}
```

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `edit` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
