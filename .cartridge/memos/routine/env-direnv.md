---
kind: routine
description: Load and unload environment variables per directory with direnv — an .envrc that activates
  on cd into a project and reverts on the way out. Use to scope env vars, PATH additions, or secrets to
  one project without polluting your global shell.
uses:
- usage: '[[run-usage]]'
  when:
  - per project environment variables
  - load env on cd into a directory
  - scope PATH to a project
  - automatic dotenv
  - activate a virtualenv on cd
  - project-local secrets
  tags:
  - env
  - direnv
  - envrc
  - project
  - dotenv
  - scope
---

## Inputs

Requires on PATH: `direnv`. Recipe parameters are named in Do; supply paths and arguments for the current task.
Risk: side effects shell-state; danger low.

## Do

Per-directory environment: an `.envrc` in a project is sourced when you `cd` in
and unloaded when you leave, so PATH additions, env vars, and tool versions stay
scoped to that project. It hooks the shell (one line in your rc — see
[[shell-env]] for where that goes). Because an `.envrc` runs code, direnv blocks it
until you `allow` it — the security gate.

| Recipe | Does |
|--------|------|
| `allow` | trust the current `.envrc` so it loads (required after any edit) |
| `edit` | open `.envrc` in `$EDITOR` and re-allow on save |
| `reload` | force a reload of the current environment |
| `status` | show direnv's state and which `.envrc` files are loaded |

`.envrc` is shell, plus helpers: `export FOO=bar`, `PATH_add ./bin` (prepend a
project dir), `dotenv` (load a `.env` file), `layout python` / `use node 20`
(activate a runtime). **Every edit re-locks it** — direnv refuses to load an
`.envrc` whose contents changed until you `allow` again, so a malicious repo
cannot run code on `cd`. Add `.envrc` to git but keep secrets in a gitignored
`.env` loaded via `dotenv`.

```just
# trust the current directory's .envrc so it loads (default)
allow:
  direnv allow

# edit .envrc and re-allow it on save
edit:
  direnv edit .

# force a reload of the current environment
reload:
  direnv reload

# show direnv state and which .envrc files are active
status:
  direnv status
```

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `allow` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
