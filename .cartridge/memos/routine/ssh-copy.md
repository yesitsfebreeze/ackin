---
kind: routine
description: Copy files over SSH with scp — push a local path to a remote host, pull a remote path down,
  or copy between two remotes. Use for one-shot file transfers over an SSH connection.
uses:
- usage: '[[run-usage]]'
  when:
  - scp a file
  - copy a file to a remote host
  - download a file from a server
  - upload over ssh
  - copy file between two remotes
  - transfer files over ssh
  tags:
  - ssh
  - scp
  - copy
  - transfer
  - file
---

## Inputs

Requires on PATH: `ssh`. Recipe parameters are named in Do; supply paths and arguments for the current task.
Risk: side effects network; danger low.

## Do

`scp` transfers, one shot. `to` pushes local → remote, `from` pulls remote →
local, `between` copies remote → remote. Remote paths are `host:path` where
`host` resolves through [[ssh-config]]; `-r` makes any of them recurse into a
directory. For repeated or incremental syncs prefer [[ssh-sync]] (rsync only sends
the diff).

| Recipe | Direction | Args |
|--------|-----------|------|
| `to` | local → remote | `src`, `host`, `dest` |
| `from` | remote → local | `host`, `src`, `dest` |
| `between` | remote → remote | `src_host:src`, `dst_host:dst` |

`recurse` defaults on (`-r`); a single file copies fine either way. Each path is
quoted independently so a space or glob in a name can't break the command apart.

```just
# push a local path to host:dest (default)
to src host dest recurse="-r":
  scp {{quote(recurse)}} {{quote(src)}} {{quote(host + ":" + dest)}}

# pull host:src down to a local dest
from host src dest recurse="-r":
  scp {{quote(recurse)}} {{quote(host + ":" + src)}} {{quote(dest)}}

# copy directly between two remotes (each as host:path)
between src dst recurse="-r":
  scp {{quote(recurse)}} {{quote(src)}} {{quote(dst)}}
```

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `to` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
