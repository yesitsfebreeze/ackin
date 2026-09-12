---
kind: routine
description: Order lines with sort — lexical or numeric, by a chosen key field, reverse, unique, and human-readable
  sizes. Half of the sort | uniq -c | sort -rn frequency idiom. Use to order output, sort by a column,
  or dedupe while ordering.
uses:
- usage: '[[run-usage]]'
  when:
  - sort lines
  - sort numerically
  - sort by a column or field
  - reverse sort
  - sort and dedupe
  - sort by file size
  - order output before uniq
  tags:
  - text
  - sort
  - order
  - numeric
  - key
  - coreutils
---

## Inputs

Requires on PATH: `coreutils`. Recipe parameters are named in Do; supply paths and arguments for the current task.
Risk: danger none.

## Do

Order lines from a file or stdin. Default is lexical (dictionary) order — the
classic trap, since it puts `10` before `2`; use `numeric` (`-n`) for numbers.
`key` sorts on a chosen field; `unique` (`-u`) collapses duplicates inline. The
canonical pairing is `sort | uniq -c | sort -rn` for a frequency table — see
[[text-uniq]] for the count half. Lives next to [[text-cut]] for picking the field
first.

| Recipe | Flags | Orders by |
|--------|-------|-----------|
| `run` | — | lexical (dictionary) order |
| `numeric` | `-n` (`-rn` reverse) | numeric value |
| `key` | `-k N` | the Nth whitespace field |
| `unique` | `-u` | sorted, with duplicates removed |

Flags worth knowing: `-r` reverse, `-h` human-numeric (`2K` < `1M`), `-V`
version order (`1.9` < `1.10`), `-f` fold case, `-t` set the field separator,
`-s` stable. `-k 2,2n` sorts on field 2 numerically only. Always `-n` when the
data is numbers — lexical order silently mis-sorts them.

```just
# lexical (dictionary) order (default)
run file="-":
  sort -- {{quote(file)}}

# numeric order; pass flags="-rn" for descending
numeric flags="-n" file="-":
  sort {{flags}} -- {{quote(file)}}

# sort on the Nth whitespace-separated field
key n file="-":
  sort -k {{quote(n)}} -- {{quote(file)}}

# sorted output with duplicate lines removed
unique file="-":
  sort -u -- {{quote(file)}}
```

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `run` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
