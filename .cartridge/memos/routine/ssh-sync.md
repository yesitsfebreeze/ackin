---
kind: routine
description: Sync directories over SSH with rsync — push a local tree to a remote, pull one down, or mirror
  so the destination matches the source exactly. Use for incremental, repeatable transfers.
uses:
- usage: '[[run-usage]]'
  when:
  - rsync to a remote
  - sync a directory over ssh
  - pull a remote directory
  - mirror a folder to a server
  - deploy files with rsync
  - incremental file sync
  tags:
  - ssh
  - rsync
  - sync
  - mirror
  - deploy
---

## Inputs

Requires on PATH: `ssh`, `rsync`. Recipe parameters are named in Do; supply paths and arguments for the current task.
Risk: side effects network; danger low.

## Do

`rsync -avz` over an SSH transport — archive mode (perms, times, symlinks),
verbose, compressed. Only the delta crosses the wire, so re-running is cheap;
that is the difference from [[ssh-copy]]'s full scp. `host` resolves through
[[ssh-config]].

| Recipe | Direction | Danger |
|--------|-----------|--------|
| `push` | local → remote | low |
| `pull` | remote → local | low |
| `mirror` | local → remote, **`--delete`** | medium |

A trailing slash on `src` matters: `dir/` copies the *contents* into `dest`,
`dir` copies the directory itself under `dest`.

**`mirror` deletes.** `--delete` removes anything on the destination that is not
in the source, so a wrong `src` or a missing slash can wipe remote files. Dry-run
first: append `--dry-run` to the recipe's flags, or run `push` and confirm before
reaching for `mirror`.

```just
# push local src to host:dest, incremental (default)
push src host dest:
  rsync -avz -e ssh {{quote(src)}} {{quote(host + ":" + dest)}}

# pull host:src down to a local dest, incremental
pull host src dest:
  rsync -avz -e ssh {{quote(host + ":" + src)}} {{quote(dest)}}

# mirror: make host:dest match src exactly, deleting extras on the remote
mirror src host dest:
  rsync -avz --delete -e ssh {{quote(src)}} {{quote(host + ":" + dest)}}
```

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `push` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
