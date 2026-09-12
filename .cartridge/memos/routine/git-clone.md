---
kind: routine
description: Copy a repository with git clone — a full clone, a shallow clone for speed, a single branch,
  or one that pulls submodules. Use to grab a repo, fetch only recent history to save time and space,
  or check out a specific branch on clone.
uses:
- usage: '[[run-usage]]'
  when:
  - clone a repository
  - shallow clone
  - clone a specific branch
  - clone with submodules
  - copy a repo quickly
  - download a git project
  - save space cloning
  tags:
  - git
  - clone
  - copy
  - shallow
  - submodule
  - vcs
---

## Inputs

Requires on PATH: `git`. Recipe parameters are named in Do; supply paths and arguments for the current task.
Risk: side effects network, file-write; danger low.

## Do

Copy a remote repository to a local working tree. `get` is the full clone;
`shallow` truncates history to save time and bandwidth on large repos; `branch`
checks out a specific branch (and only it); `submodules` also fetches nested
repos. After cloning, the remote is wired as `origin` ([[git-remote]]) so push/pull
need no URL.

| Recipe | Flags | Does |
|--------|-------|------|
| `get` | — | full clone with complete history |
| `shallow` | `--depth 1` | only the latest commit (fast, small) |
| `branch` | `-b --single-branch` | clone just one branch |
| `submodules` | `--recurse-submodules` | also clone nested submodule repos |

`--depth 1` (shallow) is the big time-saver on a huge or deep repo when you only
need the current code — CI uses it constantly. Its tradeoff: no full history, so
`git log` is truncated; `git fetch --unshallow` later restores it. Forgetting
`--recurse-submodules` leaves submodule directories empty — fix after the fact
with `git submodule update --init --recursive`. Pass a second argument to name
the target directory.

```just
# full clone with complete history (default); dir optional
get url dir="":
  git clone {{quote(url)}} {{dir}}

# shallow clone — only the latest commit (fast, small)
shallow url dir="":
  git clone --depth 1 {{quote(url)}} {{dir}}

# clone a single named branch only
branch name url dir="":
  git clone -b {{quote(name)}} --single-branch {{quote(url)}} {{dir}}

# clone and also fetch submodules
submodules url dir="":
  git clone --recurse-submodules {{quote(url)}} {{dir}}
```

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `get` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
