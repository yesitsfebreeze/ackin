---
kind: routine
description: Run things over SSH — execute one command on a remote host, open an interactive shell, or
  set up a port-forward tunnel. Use to drive a remote machine without leaving the local shell.
uses:
- usage: '[[run-usage]]'
  when:
  - run a command on a remote host
  - ssh into a server
  - execute remote command
  - open an interactive ssh shell
  - forward a port over ssh
  - ssh tunnel
  - port forwarding
  tags:
  - ssh
  - remote
  - exec
  - shell
  - tunnel
---

## Inputs

Requires on PATH: `ssh`. Recipe parameters are named in Do; supply paths and arguments for the current task.
Risk: side effects network; danger low.

## Do

Three entry points onto a remote host. `run` executes a single command and
returns its output; `shell` drops into an interactive session; `tunnel` forwards
a port. `host` is whatever `ssh` resolves — a bare name, `user@host`, or a `Host`
alias from [[ssh-config]] (which is where `User`, `Port`, and key selection live).

| Recipe | Does | `host` form |
|--------|------|-------------|
| `run` | one command, capture output | `user@host` or alias |
| `shell` | interactive login session | same |
| `tunnel` | forward `local:remote:port` | same |

`tunnel` defaults to local forwarding (`-L` — `localhost:LPORT` reaches
`RHOST:RPORT` through the server); pass `dir="R"` for reverse (`-R`). It stays in
the foreground holding the tunnel open until interrupted.

The remote command is passed as a **single argument** so the local shell never
splits or expands it — it arrives at the remote intact and is parsed only by the
*remote* shell. Quote anything inside it for the remote side yourself.

```just
# execute one command on the remote host, return its output (default)
run host cmd:
  ssh {{quote(host)}} {{quote(cmd)}}

# open an interactive login shell on the remote host
shell host:
  ssh -t {{quote(host)}}

# forward a port: dir L = -L lport:rhost:rport, dir R = -R (reverse)
tunnel host lport rhost rport dir="L":
  #!/usr/bin/env sh
  set -eu
  h={{quote(host)}}; lp={{quote(lport)}}; rh={{quote(rhost)}}; rp={{quote(rport)}}; d={{quote(dir)}}
  spec="$lp:$rh:$rp"
  case "$d" in
    R) ssh -N -R "$spec" "$h" ;;
    *) ssh -N -L "$spec" "$h" ;;
  esac
```

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `run` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
