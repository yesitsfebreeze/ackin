---
kind: routine
description: Create and inspect links with ln — make a symbolic link, read where one points, replace an
  existing link atomically, and make a hard link. Use to symlink a config into place, see a link's target,
  or repoint an existing symlink.
uses:
- usage: '[[run-usage]]'
  when:
  - create a symlink
  - make a symbolic link
  - where does this link point
  - repoint a symlink
  - replace an existing link
  - hard link a file
  - link a dotfile into place
  tags:
  - file
  - ln
  - symlink
  - link
  - hardlink
  - target
---

## Inputs

Requires on PATH: `coreutils`. Recipe parameters are named in Do; supply paths and arguments for the current task.
Risk: side effects file-write; danger medium.

## Do

Point one path at another. `symlink` makes a symbolic link (a path that refers
to another path); `target` reads where a link points; `repoint` atomically
replaces an existing link; `hard` makes a hard link (a second name for the same
inode). Symlinks are what dotfile managers use to put a config in place from a
repo. The argument order is `ln -s TARGET LINKNAME` — target first, the new name
second (easy to reverse).

| Recipe | Does |
|--------|------|
| `symlink` | create a symbolic link pointing at a target |
| `target` | print where an existing symlink points |
| `repoint` | atomically replace an existing symlink (`-sf`) |
| `hard` | create a hard link (same inode, no target path) |

Use an **absolute** target unless you deliberately want a relative link — a
relative symlink resolves against the *link's* directory, so it breaks if moved.
`repoint` uses `-sfn`: `-f` replaces, `-n` stops it from descending into an
existing link-to-a-directory (the classic footgun that nests a link inside the
old target). Hard links share storage and survive the original's deletion but
cannot cross filesystems or link directories; symlinks can do both.

```just
# create a symbolic link: target first, new link name second (default)
symlink target linkname:
  ln -s {{quote(target)}} {{quote(linkname)}}

# print where an existing symlink points
target link:
  readlink -f {{quote(link)}}

# atomically replace an existing symlink to point somewhere new
repoint target linkname:
  ln -sfn {{quote(target)}} {{quote(linkname)}}

# create a hard link (a second name for the same inode)
hard target linkname:
  ln {{quote(target)}} {{quote(linkname)}}
```

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `symlink` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
