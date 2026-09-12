---
kind: routine
description: Find files and directories by name with fd — match a name pattern, filter by extension or
  type, run a command per result. Use to locate paths fast, respecting .gitignore.
uses:
- usage: '[[run-usage]]'
  when:
  - find a file by name
  - locate paths
  - list files by extension
  - find directories
  - run a command on every match
  tags:
  - search
  - fd
  - find
  - files
  - paths
---

## Inputs

Requires on PATH: `fd`. Recipe parameters are named in Do; supply paths and arguments for the current task.
Risk: danger none.

## Do

Path search by name or regex, walking the tree while skipping `.gitignore` and
hidden entries by default. `find` is the default; the rest narrow by extension,
type, or fan a command out per result. Matching is smart-case regex (add `-g`
for globs) — see [[search-patterns]] for the rules and the shared walk flags
(`-H` hidden, `-I` ignored, `-g`). Pair with [[search-rg]] to grep inside the paths
fd turns up.

| Recipe | Flag | Selects |
|--------|------|---------|
| `find` | — | paths whose name matches the pattern |
| `ext` | `-e` | files with a given extension |
| `type` | `-t f`/`-t d` | files only / directories only |
| `exec` | `-x` | run a command once per matching path |

In `exec`, fd substitutes `{}` with each path (`{/}` basename, `{//}` parent,
`{.}` without extension). An empty pattern matches everything under the root.

```just
# find paths by name pattern under a root (default)
find pattern root=".":
  fd {{quote(pattern)}} {{quote(root)}}

# only files with this extension (e.g. rs, md, json)
ext extension root=".":
  fd -e {{quote(extension)}} . {{quote(root)}}

# files only (f) or directories only (d)
type kind root=".":
  fd -t {{quote(kind)}} . {{quote(root)}}

# run a command per match; trailing args form the command, {} is each path
exec pattern root +command:
  fd {{quote(pattern)}} {{quote(root)}} -x {{command}}
```

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `find` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
