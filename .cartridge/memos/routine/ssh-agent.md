---
kind: routine
description: Manage loaded SSH keys with ssh-agent and ssh-add — load a key so you type its passphrase
  once, list the loaded keys, and remove them. Use to stop being prompted for a key passphrase every connection,
  or to check which keys the agent holds.
uses:
- usage: '[[run-usage]]'
  when:
  - stop being asked for my ssh passphrase
  - add a key to the ssh agent
  - list loaded ssh keys
  - ssh-add
  - load a private key
  - remove keys from the agent
  tags:
  - ssh
  - agent
  - ssh-add
  - keys
  - passphrase
  - identity
---

## Inputs

Requires on PATH: `ssh`. Recipe parameters are named in Do; supply paths and arguments for the current task.
Risk: side effects shell-state; danger low.

## Do

The agent holds your decrypted private keys in memory so you enter a passphrase
once per session, not once per connection. `add` loads a key; `list` shows what
is loaded; `start` launches an agent in the current shell; `clear` unloads
everything. Keys are looked up by [[ssh-key]] and used by [[ssh-run]], [[ssh-copy]], and
[[ssh-tunnel]] — once a key is added, all of them stop prompting.

| Recipe | Does |
|--------|------|
| `add` | load a private key (prompts for its passphrase once) |
| `list` | list the keys currently held by the agent |
| `start` | start an ssh-agent and export its env into this shell |
| `clear` | remove all keys from the agent |

`ssh-add` with no path loads the default keys (`~/.ssh/id_*`). "Could not open a
connection to your authentication agent" means no agent is running — `start`
fixes it (most desktops start one at login). On macOS, `ssh-add --apple-use-keychain
key` stores the passphrase in the Keychain so it survives reboots. `add -t 1h`
loads a key with a lifetime, auto-removing it after the timeout.

```just
# load a private key, prompting for its passphrase once (default ~/.ssh keys)
add key="":
  ssh-add {{key}}

# list the keys currently held by the agent
list:
  ssh-add -l

# start an ssh-agent and export its env into this shell
start:
  eval "$(ssh-agent -s)"

# remove all keys from the agent
clear:
  ssh-add -D
```

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `add` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
