---
kind: routine
description: Compress or decompress a single file with gzip, zstd, or xz — the stream compressors behind
  .gz / .zst / .xz. Use to shrink one file (or feed a tar stream), not to bundle many files.
uses:
- usage: '[[run-usage]]'
  when:
  - gzip a file
  - compress a file
  - decompress a gz
  - zstd compress
  - xz compress
  - shrink a file
  tags:
  - archive
  - compress
  - gzip
  - zstd
  - xz
---

## Inputs

Requires on PATH: `gzip`, `zstd`, `xz`. Recipe parameters are named in Do; supply paths and arguments for the current task.
Risk: side effects file-write; danger low.

## Do

These compress one stream; they do **not** bundle directories — pair them with
[[archive-tar]] for that (the `.tar.gz` / `.tar.zst` / `.tar.xz` pattern). Pick the
codec by trade-off: gzip is universal, zstd is fast at high ratios, xz packs
smallest but slowest. See [[archive-formats]] for the extension map.

| Recipe | Tool | Direction |
|--------|------|-----------|
| `gzip` | `gzip` | file → `.gz` |
| `gunzip` | `gunzip` | `.gz` → file |
| `zstd` | `zstd` / `unzstd` | `.zst` ↔ file |
| `xz` | `xz` / `unxz` | `.xz` ↔ file |

Every recipe passes `-k` to keep the original; drop it to replace in place. Each
compressor's matching decompressor undoes it (`gunzip`, `unzstd`, `unxz`).

```just
# compress a file to .gz, keeping the original (default)
gzip file:
  gzip -k -- {{quote(file)}}

# decompress a .gz back to the file, keeping the .gz
gunzip file:
  gunzip -k -- {{quote(file)}}

# compress (default) or decompress a file with zstd; mode = c | d
zstd file mode="c":
  #!/usr/bin/env sh
  set -eu
  f={{quote(file)}}
  m={{quote(mode)}}
  case "$m" in
    c) zstd -k -- "$f" ;;
    d) unzstd -k -- "$f" ;;
    *) echo "mode must be c or d" >&2; exit 1 ;;
  esac

# compress (default) or decompress a file with xz; mode = c | d
xz file mode="c":
  #!/usr/bin/env sh
  set -eu
  f={{quote(file)}}
  m={{quote(mode)}}
  case "$m" in
    c) xz -k -- "$f" ;;
    d) unxz -k -- "$f" ;;
    *) echo "mode must be c or d" >&2; exit 1 ;;
  esac
```

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `gzip` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
