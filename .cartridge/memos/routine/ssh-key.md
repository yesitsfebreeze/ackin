---
kind: routine
description: Manage SSH keys — generate an ed25519 keypair, install a public key on a remote host, add
  a key to the agent, or list local keys. Use to set up or inspect key-based authentication.
uses:
- usage: '[[run-usage]]'
  when:
  - generate an ssh key
  - create ssh keypair
  - ssh-keygen
  - copy public key to server
  - ssh-copy-id
  - add key to ssh agent
  - ssh-add
  - list ssh keys
  tags:
  - ssh
  - key
  - keygen
  - agent
  - auth
---

## Inputs

Requires on PATH: `ssh`. Recipe parameters are named in Do; supply paths and arguments for the current task.
Risk: side effects network; danger low.

## Do

Set up and inspect key-based auth. `gen` makes an ed25519 keypair, `copy`
installs the public half on a host, `add` loads a key into the running agent, and
`list` shows what's in `~/.ssh`. The host file an installed key authorizes against
and `IdentityFile` selection both live in [[ssh-config]].

| Recipe | Tool | Effect |
|--------|------|--------|
| `gen` | `ssh-keygen -t ed25519` | write keypair to `path` |
| `copy` | `ssh-copy-id` | append pubkey to remote `authorized_keys` |
| `add` | `ssh-add` | load key into the agent for this session |
| `list` | `ls ~/.ssh` | list key files present |

`gen` writes to `~/.ssh/id_ed25519` by default and refuses to clobber an existing
key. `copy` is the only one that touches the network; the rest are local.

```just
# generate an ed25519 keypair at path (default)
gen path="~/.ssh/id_ed25519" comment="":
  ssh-keygen -t ed25519 -f {{quote(path)}} -C {{quote(comment)}}

# install the local public key onto a remote host
copy host:
  ssh-copy-id {{quote(host)}}

# add a private key to the running ssh-agent
add path="~/.ssh/id_ed25519":
  ssh-add {{quote(path)}}

# list key files in ~/.ssh
list:
  ls -l ~/.ssh
```

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `gen` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
