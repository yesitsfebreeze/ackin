---
kind: routine
description: Look up live cheat-sheets over curl with cht.sh — terse, example-first answers for both unix
  commands and programming questions ("how do I reverse a list in python"). One curl per query, no install,
  always current. Use to recall a command's flags or a language idiom fast.
uses:
- usage: '[[run-usage]]'
  when:
  - how do I do X in some language
  - syntax reminder
  - command flags
  - code idiom
  - one-liner for a language or CLI
  - quick cheat sheet
  - cht.sh lookup
  tags:
  - help
  - cht.sh
  - cheatsheet
  - examples
  - syntax
  - idiom
  - curl
---

## Inputs

Requires on PATH: `curl`. Recipe parameters are named in Do; supply paths and arguments for the current task.
Risk: danger none.

## Do

A cheat-sheet service you hit with plain `curl` — no client to install. Two
modes: `sheet` answers for a **unix command** (`tar`, `ssh`), `lang` answers a
**programming question** in a language (`python` / "read a file"). Answers are a
few real examples, not a man page. Because it's fetched live it never goes stale
— the trade-off versus [[help-tldr]]'s offline cache.

| Recipe | Does |
|--------|------|
| `sheet` | cheat-sheet for a unix command (default) |
| `lang` | example for a question in a language |
| `search` | filter a command's sheet by keyword (`~`) |
| `list` | list every available topic |

Queries are URL paths, so **spaces become `+`** (the recipes do this for you).
Append options to the URL to shape output: `?Q` strips the explanatory comments
(code only), `?T` strips ANSI colors (best when piping or feeding to a tool),
`?QT` for both, `?style=bw` for a plain theme. The reflex: resolve a memo
first; if none fits, `cht` the syntax rather than guessing.

```just
# cheat-sheet for a unix command, e.g. `sheet tar` (default)
sheet command:
  curl -s {{quote("cht.sh/" + command)}}

# a programming idiom: `lang python "reverse a list"` — spaces become +
lang language query:
  curl -s "cht.sh/{{language}}/$(printf %s {{quote(query)}} | tr ' ' '+')"

# filter a command's sheet to lines matching a keyword (the ~ search)
search command keyword:
  curl -s {{quote("cht.sh/" + command + "~" + keyword)}}

# list every topic cht.sh knows
list:
  curl -s cht.sh/:list
```

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `sheet` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
