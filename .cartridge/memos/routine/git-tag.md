---
kind: routine
description: Mark commits with git tags — create an annotated release tag, list existing tags, push tags
  to the remote, and delete one. Use to tag a release, see what versions exist, or publish a tag so CI
  and others can see it.
uses:
- usage: '[[run-usage]]'
  when:
  - tag a release
  - create a version tag
  - list git tags
  - push a tag to the remote
  - delete a tag
  - annotated tag
  - mark a commit as a version
  tags:
  - git
  - tag
  - release
  - version
  - annotate
  - vcs
---

## Inputs

Requires on PATH: `git`. Recipe parameters are named in Do; supply paths and arguments for the current task.
Risk: side effects git-write; danger low.

## Do

A permanent name for a commit — almost always a release version. `create` makes
an **annotated** tag (with a message, author, and date — what releases should
use); `list` shows existing tags; `push` publishes them (a plain `git push` does
**not** send tags); `delete` removes one. Tag names and the commit-ish they point
at follow [[git-refs]]; a release routine builds on these.

| Recipe | Does |
|--------|------|
| `create` | annotated tag at HEAD (or a given commit) with a message |
| `list` | list tags (newest first, optionally filtered) |
| `push` | push one tag — or all tags — to the remote |
| `delete` | delete a tag locally (and note how to delete it remotely) |

Prefer **annotated** (`-a`) over lightweight tags for releases — they store who/
when/why and are what `git describe` and release tooling expect. The gotcha:
tags are not pushed by `git push`; you must `git push origin <tag>` (or
`--tags`). To delete on the remote, push an empty ref: `git push origin :v1.2.0`.
`git tag -l 'v1.*'` filters; `git describe --tags` names the nearest tag.

```just
# annotated tag at a commit (default HEAD) with a message
create name message commit="HEAD":
  git tag -a {{quote(name)}} -m {{quote(message)}} {{quote(commit)}}

# list tags, newest first (pass pattern like "v1.*" to filter)
list pattern="*":
  git tag -l {{quote(pattern)}} --sort=-creatordate

# push one tag to the remote (use name="--tags" to push all)
push name remote="origin":
  git push {{quote(remote)}} {{name}}

# delete a tag locally (remote: git push origin :refs/tags/NAME)
delete name:
  git tag -d {{quote(name)}}
```

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `create` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
