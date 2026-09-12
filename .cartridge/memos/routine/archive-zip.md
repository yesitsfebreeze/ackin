---
kind: routine
description: Build and read .zip archives — create recursively, list contents, extract to a directory,
  add files to an existing zip. Use for the cross-platform .zip format that Windows and macOS open natively.
uses:
- usage: '[[run-usage]]'
  when:
  - make a zip
  - zip a folder
  - list a zip
  - extract a zip
  - unzip a file
  - add a file to a zip
  tags:
  - archive
  - zip
  - unzip
  - compression
  - cross-platform
---

## Inputs

Requires on PATH: `zip`, `unzip`. Recipe parameters are named in Do; supply paths and arguments for the current task.
Risk: side effects file-write; danger low.

## Do

The portable archive every desktop OS opens unaided — unlike tar it both bundles
and compresses per-entry. `create` is the default; reading uses `unzip`. For
unix-native bundling pick [[archive-tar]]; see [[archive-formats]] for choosing between
them.

| Recipe | Tool | Does |
|--------|------|------|
| `create` | `zip -r` | recurse paths into a new zip |
| `list` | `unzip -l` | print the entry table, no extraction |
| `extract` | `unzip -d` | unpack into a target dir (writes files) |
| `add` | `zip -g` | grow an existing zip with more paths |

`zip -r` recurses directories; `add` (`-g`) appends in place. `extract` writes
under `dir`, creating it if needed.

```just
# recurse paths into a new zip archive (default)
create archive +paths:
  zip -r {{quote(archive)}} {{paths}}

# list entries in a zip, without extracting
list archive:
  unzip -l {{quote(archive)}}

# extract a zip into a directory (created if missing)
extract archive dir=".":
  #!/usr/bin/env sh
  set -eu
  a={{quote(archive)}}
  d={{quote(dir)}}
  mkdir -p "$d"
  unzip "$a" -d "$d"

# add paths to an existing zip in place
add archive +paths:
  zip -g {{quote(archive)}} {{paths}}
```

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `create` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
