---
kind: note
description: Cross-shell translation table — the same idiom (assign, export, substitute, condition, loop,
  list, glob) written in bash, zsh, fish, and nu side by side. The wizard's lookup when you know what
  you want in one shell and need it in another.
uses:
- usage: '[[read-usage]]'
  when:
  - how do I do this bash thing in fish
  - convert bash to nushell
  - export variable in each shell
  - command substitution across shells
  - loop syntax in fish vs bash
  - same idiom different shell
  - powershell where-object
  - pwsh filter output
  - select-object property
  - powershell variable syntax
  - foreach-object
  - powershell script
  - verb-noun cmdlet
  tags:
  - shell
  - rosetta
  - translation
  - bash
  - zsh
  - fish
  - nu
  - pwsh
  - powershell
  - object-pipeline
  - cmdlet
  - dotnet
  - cross-shell
---

# Rosetta

One idiom, four shells. When you know the bash form and land in fish or nu — or
the reverse — read across the row. Per-shell depth lives in [[shell-bash]],
[[shell-zsh]], [[shell-fish]], [[shell-nu]]. PowerShell has no depth memo — the
tables below stay four-column and its whole payload sits in one section at the
end, because filling a fifth column would mean inventing cells this record
never measured.

## Variables and environment

| Idiom | bash / zsh | fish | nu |
|-------|-----------|------|-----|
| assign | `x=1` | `set x 1` | `let x = 1` |
| reassign | `x=2` | `set x 2` | `mut x = 1; $x = 2` |
| export | `export X=1` | `set -x X 1` | `$env.X = "1"` |
| read | `$x` / `${x}` | `$x` | `$x` |
| default | `${x:-d}` | `set -q x; or set x d` | `$x | default "d"` |
| persist across sessions | edit rc | `set -U x v` | edit `env.nu` |

## Substitution and pipes

| Idiom | bash / zsh | fish | nu |
|-------|-----------|------|-----|
| command sub | `$(cmd)` | `(cmd)` | `(cmd)` |
| capture lines | `mapfile -t a < <(cmd)` | `set a (cmd)` | `let a = (cmd \| lines)` |
| pipe | `a \| b` | `a \| b` | `a \| b` (typed values) |
| and / or | `a && b` / `a \|\| b` | `a; and b` / `a; or b` | `a; b` / `if` |

## Conditionals

| Idiom | bash / zsh | fish | nu |
|-------|-----------|------|-----|
| if file | `if [[ -f f ]]; then …; fi` | `if test -f f; …; end` | `if ($f \| path exists) { … }` |
| string eq | `[[ "$a" == "$b" ]]` | `test "$a" = "$b"` | `$a == $b` |
| regex | `[[ $s =~ re ]]` | `string match -rq re $s` | `$s =~ 're'` |
| numeric | `(( a > b ))` | `test $a -gt $b` | `$a > $b` |

## Loops and lists

| Idiom | bash / zsh | fish | nu |
|-------|-----------|------|-----|
| list literal | `arr=(a b c)` | `set arr a b c` | `[a b c]` |
| index | `${arr[0]}` (bash) / `${arr[1]}` (zsh) | `$arr[1]` | `$arr.0` |
| length | `${#arr[@]}` | `count $arr` | `$arr \| length` |
| for | `for x in "${arr[@]}"; do …; done` | `for x in $arr; …; end` | `for x in $arr { … }` |
| map | `for x in …; do f "$x"; done` | `for x in …; f $x; end` | `$arr \| each { \|x\| f $x }` |
| filter | `grep`/`[[ ]]` in loop | `string match` in loop | `$arr \| where { … }` |

## Functions

| Idiom | bash / zsh | fish | nu |
|-------|-----------|------|-----|
| define | `f() { …; }` | `function f; …; end` | `def f [] { … }` |
| arg | `"$1"` | `$argv[1]` (or `-a name`) | named param `def f [name] {…}` |
| local | `local x` | `set -l x` | `let` (always scoped) |
| return value | `echo` + `$(f)` | `echo` + `(f)` | last expression is the value |

## The mental model shift

bash/zsh/fish all stream **text** between commands — the receiver re-parses.
Nu streams **typed values** ([[shell-nu]]): a table stays a table, so you `where`
and `get` instead of `grep` and `cut`. That is the one row that doesn't
translate — it's a different paradigm, not a different syntax.

## PowerShell

Folded in from `shell-pwsh` on 2026-09-12. Verb-Noun cmdlets, and the pipeline
carries .NET objects with typed properties rather than text — the same paradigm
family as nu, so the mental model shift above has a third member. The
cross-platform binary is `pwsh`.

| Cmdlet (aliases) | Does |
|------------------|------|
| `Get-ChildItem` (`gci`, `ls`) | list files as `FileInfo` objects |
| `Where-Object { $_.X -gt 1 }` (`where`, `?`) | filter rows |
| `Select-Object a,b` (`select`) | project properties |
| `Sort-Object X` (`sort`) | sort by property |
| `ForEach-Object { ... }` (`foreach`, `%`) | map a block over the pipe |
| `Measure-Object` | count, sum, average |
| `Group-Object X` | aggregate by property |

`$_` (or `$PSItem`) is the current pipeline object inside a block. The worked
pipeline, note the typed `5MB` literal:

```powershell
Get-ChildItem -Recurse -Filter *.log |
  Where-Object { $_.Length -gt 5MB } |
  Sort-Object LastWriteTime |
  Select-Object Name, Length
```

| Form | Is |
|------|-----|
| `$x = 5` | assignment, any type |
| `$env:PATH` | environment variable |
| `-eq -ne -gt -lt -ge -le` | comparison operators |
| `-match` / `-replace` | regex test / substitute |
| `-like` | wildcard match (`*`, `?`) |
| `-contains` / `-in` | membership |
| `@(1,2,3)` / `@{k=1}` | array / hashtable |
| `if ($x -gt 0) { "pos" } elseif ($x -eq 0) { "zero" } else { "neg" }` | branch |
| `foreach ($f in Get-ChildItem) { $f.Name }` | iterate |
| `function Get-Greeting { param([string]$Name) "hi $Name" }` | define |

Gotchas:

- `>` redirects and is never "greater than" — use `-gt`.
- A single returned object is not an array. Wrap in `@(...)` to force one.
- Output stays objects until the console or `Out-String`. `Format-Table` and
  every other `Format-*` is display-only — never pipe one into further
  processing.
- `=` assigns and `-eq` compares, reversed from most languages' muscle memory.
