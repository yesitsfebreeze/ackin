---
kind: routine
description: Locate a command and tell what it is — find the binary a name resolves to, test whether a
  command exists in a script, and reveal aliases, functions, or builtins behind a name. Use to find where
  a tool lives, check if it is installed, or see why a name runs something unexpected.
uses:
- usage: '[[run-usage]]'
  when:
  - where is this binary
  - is a command installed
  - what does this command name resolve to
  - check if a tool exists in a script
  - is it an alias or a builtin
  - find a program on PATH
  tags:
  - shell
  - which
  - type
  - command
  - path
  - installed
---

## Inputs

Requires on PATH: `coreutils`. Recipe parameters are named in Do; supply paths and arguments for the current task.
Risk: danger none.

## Do

Resolve a command name. `locate` finds the executable on `$PATH`; `exists` is
the script-safe "is it installed" test; `what` reveals whether a name is an
alias, function, builtin, or file; `all` shows every match (shadowing). The
portable, script-correct test is `command -v`, not `which` — `which` is an
external that is missing on some minimal systems and does not see shell
functions. See [[shell-env]] for the `$PATH` it searches.

| Recipe | Tool | Does |
|--------|------|------|
| `locate` | `command -v` | the path/definition a name resolves to |
| `exists` | `command -v … >/dev/null` | exit 0 if the command exists (for `if`) |
| `what` | `type` | classify: alias / function / builtin / file |
| `all` | `type -a` | every definition, in resolution order |

In a script, guard optional tools with `if command -v fzf >/dev/null; then …` —
it is POSIX, sees builtins and functions, and needs no external. `type foo`
explains a surprise (why `ls` is colored: it is aliased); `type -a foo` shows the
shadowing when a PATH entry hides the system binary. fish uses `type`/`command
-v` too; nu has `which`.

```just
# the path or definition a command name resolves to (default)
locate name:
  command -v {{quote(name)}}

# exit 0 if the command exists, non-zero if not (for use in if/&&)
exists name:
  command -v {{quote(name)}} >/dev/null 2>&1

# classify a name: alias / function / builtin / file
what name:
  type {{quote(name)}}

# every definition of a name, in resolution order (shows shadowing)
all name:
  type -a {{quote(name)}}
```

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `locate` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
