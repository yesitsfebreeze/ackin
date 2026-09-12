---
kind: routine
description: Tidy text layout with the small coreutils — align delimited data into columns, wrap long
  lines to a width, and convert tabs to spaces or back. Use to turn ragged output into an aligned table,
  wrap prose to a column, or fix tab/space mismatches.
uses:
- usage: '[[run-usage]]'
  when:
  - align output into columns
  - make a table from delimited text
  - wrap long lines to a width
  - convert tabs to spaces
  - convert spaces to tabs
  - tidy ragged output
  tags:
  - text
  - column
  - fold
  - expand
  - align
  - wrap
---

## Inputs

Requires on PATH: `util-linux`. Recipe parameters are named in Do; supply paths and arguments for the current task.
Risk: danger none.

## Do

Layout fixes for text that is already correct but ugly. `columns` aligns
whitespace-separated fields; `table` aligns on a chosen delimiter (CSV-ish);
`wrap` breaks long lines at a width; `untab` turns tabs into spaces. These tidy
*output* — the content work belongs to [[text-awk]] and [[text-cut]].

| Recipe | Tool | Does |
|--------|------|------|
| `columns` | `column -t` | align whitespace-separated fields into columns |
| `table` | `column -t -s D` | align on a delimiter (e.g. `,` or `:`) |
| `wrap` | `fold -s -w N` | wrap lines to width N, breaking at spaces |
| `untab` | `expand` | convert tabs to spaces (`unexpand` for the reverse) |

`column -t` is the one to remember: pipe any ragged whitespace columns through
it and they snap into alignment. `-s` sets the input separator (`column -t -s:`
aligns `/etc/passwd`); add `-N name,age` for headers on modern util-linux.
`fold -s` breaks at spaces instead of mid-word. `expand -t 4` sets the tab stop;
`unexpand -a` re-tabs leading runs. None of these change content — only spacing.

```just
# align whitespace-separated fields into columns (default)
columns file="-":
  column -t -- {{quote(file)}}

# align on a delimiter (e.g. delim="," for CSV, ":" for passwd-style)
table delim file="-":
  column -t -s {{quote(delim)}} -- {{quote(file)}}

# wrap lines to width N, breaking at spaces
wrap n="80" file="-":
  fold -s -w {{quote(n)}} -- {{quote(file)}}

# convert tabs to spaces (pass width via t, default 8)
untab t="8" file="-":
  expand -t {{quote(t)}} -- {{quote(file)}}
```

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `columns` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
