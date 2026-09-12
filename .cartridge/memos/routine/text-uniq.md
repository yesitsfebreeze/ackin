---
kind: routine
description: Collapse or count adjacent duplicate lines with uniq — dedupe, prefix counts, show only repeated
  or only unique lines. Requires sorted input. The count half of the sort | uniq -c | sort -rn frequency
  idiom. Use to count occurrences or find duplicates.
uses:
- usage: '[[run-usage]]'
  when:
  - count occurrences of each line
  - dedupe sorted lines
  - find duplicate lines
  - find unique lines
  - frequency table
  - count distinct values
  tags:
  - text
  - uniq
  - dedupe
  - count
  - frequency
  - coreutils
---

## Inputs

Requires on PATH: `coreutils`. Recipe parameters are named in Do; supply paths and arguments for the current task.
Risk: danger none.

## Do

Operates on **adjacent** duplicate lines — so the input must be sorted first
([[text-sort]]), or only consecutive repeats collapse. `dedupe` removes runs;
`count` (`-c`) prefixes each line with its tally, the heart of the frequency
idiom `sort | uniq -c | sort -rn`; `repeated` and `unique` keep only the
duplicated or only the once-seen lines.

| Recipe | Flag | Keeps |
|--------|------|-------|
| `dedupe` | — | one copy of each adjacent run |
| `count` | `-c` | each distinct line, prefixed with its count |
| `repeated` | `-d` | only lines that appear more than once |
| `unique` | `-u` | only lines that appear exactly once |

The whole tool assumes sorted input — `uniq` alone on unsorted data misses
non-adjacent duplicates, the single most common mistake. The frequency recipe:
`sort file | uniq -c | sort -rn` gives a descending count of every distinct
line. `-i` folds case; `-f N` ignores the first N fields when comparing.

```just
# collapse adjacent duplicate runs (input must be sorted)
dedupe file="-":
  uniq -- {{quote(file)}}

# prefix each distinct line with its count (the frequency idiom)
count file="-":
  #!/usr/bin/env sh
  set -eu
  sort -- {{quote(file)}} | uniq -c | sort -rn

# only lines that appear more than once
repeated file="-":
  #!/usr/bin/env sh
  set -eu
  sort -- {{quote(file)}} | uniq -d

# only lines that appear exactly once
unique file="-":
  #!/usr/bin/env sh
  set -eu
  sort -- {{quote(file)}} | uniq -u
```

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `dedupe` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
