---
kind: routine
description: Copy, move, and remove files safely — copy preserving attributes, move or rename with overwrite
  protection, and delete with the guardrails that matter. Use to duplicate a file or tree, rename or relocate,
  or remove paths without the classic rm -rf mistakes.
uses:
- usage: '[[run-usage]]'
  when:
  - copy a file or directory
  - duplicate a folder
  - move or rename a file
  - delete files
  - remove a directory
  - copy preserving permissions
  - avoid overwriting on move
  tags:
  - file
  - cp
  - mv
  - rm
  - copy
  - delete
---

## Inputs

Requires on PATH: `coreutils`. Recipe parameters are named in Do; supply paths and arguments for the current task.
Risk: side effects file-write; danger high.

## Do

The everyday move/duplicate/delete trio, with the flags that prevent regret.
`copy` duplicates (preserving attributes for trees); `move` renames or relocates
with overwrite prompting; `remove` deletes; `mirror` uses [[ssh-sync]]'s rsync for
large or resumable copies. The destructive ones (`move` over an existing file,
`remove`) carry the guardrails that the bare commands omit.

| Recipe | Tool | Does |
|--------|------|------|
| `copy` | `cp -a` | copy a file or tree, preserving mode/owner/times/links |
| `move` | `mv -i` | rename/relocate, **prompting** before overwrite |
| `remove` | `rm -rI` | delete, prompting once for a recursive/bulk delete |
| `mirror` | `rsync -a` | copy a large tree, resumable, only the differences |

`cp -a` is the archive copy — recurse plus preserve everything (the plain `cp`
drops permissions and symlinks). `mv -i` and `rm -rI` add the prompts that stop
a fat-fingered overwrite or wipe: `-I` (capital) asks **once** before removing
many files or recursing, less naggy than `-i` but still a safety net. For
anything you might regret, prefer a `trash` utility over `rm`. `rsync -a` beats
`cp` for big trees — it resumes, skips unchanged files, and shows progress with
`--info=progress2`.

```just
# copy a file or tree, preserving attributes (default)
copy src dst:
  cp -a -- {{quote(src)}} {{quote(dst)}}

# move or rename, prompting before overwriting an existing file
move src dst:
  mv -i -- {{quote(src)}} {{quote(dst)}}

# remove paths, prompting once before a recursive/bulk delete
remove +paths:
  rm -rI -- {{paths}}

# mirror a large tree with rsync (resumable, copies only differences)
mirror src dst:
  rsync -a --info=progress2 {{quote(src)}} {{quote(dst)}}
```

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `copy` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
