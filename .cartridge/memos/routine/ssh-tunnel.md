---
kind: routine
description: Forward ports over SSH — a local forward to reach a remote service, a remote forward to expose
  a local one, a dynamic SOCKS proxy, and jump-host chaining. Use to reach a database behind a bastion,
  expose localhost to a server, or proxy traffic through a host.
uses:
- usage: '[[run-usage]]'
  when:
  - ssh port forward
  - reach a remote database through a bastion
  - expose localhost to a remote
  - socks proxy over ssh
  - jump host
  - tunnel a port
  - access an internal service
  tags:
  - ssh
  - tunnel
  - forward
  - port
  - socks
  - proxy
---

## Inputs

Requires on PATH: `ssh`. Recipe parameters are named in Do; supply paths and arguments for the current task.
Risk: side effects network; danger medium.

## Do

Move a port across an SSH connection. `local` (`-L`) brings a remote service to
your machine — the daily "reach the DB behind a bastion"; `remote` (`-R`)
pushes a local service out to the server; `socks` (`-D`) is a whole-proxy for a
browser; `jump` (`-J`) chains through a bastion. Connection details and aliases
live in [[ssh-config]]; once a host has a `ProxyJump`/`LocalForward` there, these
become argument-free.

| Recipe | Flag | Direction |
|--------|------|-----------|
| `local` | `-L L:host:R` | a remote `host:R` appears on your local port `L` |
| `remote` | `-R R:host:L` | your local `host:L` appears on the server's port `R` |
| `socks` | `-D port` | a local SOCKS5 proxy routing through the server |
| `jump` | `-J bastion` | connect via one or more jump hosts |

The `-L` mnemonic: `-L 5432:dbhost:5432 user@bastion` means "localhost:5432 now
reaches dbhost:5432 as seen from bastion". `-N` (in the recipes) opens the tunnel
without a shell — pure forwarding; add `-f` to background it. The forward target
is resolved *on the remote side*, so `localhost` in a `-L` means the server's
localhost, not yours. SOCKS (`-D`) plus a browser's proxy setting tunnels all
its traffic through the host.

```just
# bring a remote host:port to a local port (default): local=5432 target=db:5432
local localport target gateway:
  ssh -N -L {{quote(localport)}}:{{quote(target)}} {{quote(gateway)}}

# expose a local host:port on the server's port
remote remoteport target gateway:
  ssh -N -R {{quote(remoteport)}}:{{quote(target)}} {{quote(gateway)}}

# a local SOCKS5 proxy routing through the server
socks port gateway:
  ssh -N -D {{quote(port)}} {{quote(gateway)}}

# connect to a host via one or more jump/bastion hosts
jump bastion host:
  ssh -J {{quote(bastion)}} {{quote(host)}}
```

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `local` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
