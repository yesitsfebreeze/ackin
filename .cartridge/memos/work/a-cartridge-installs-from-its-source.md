---
kind: work
description: "A cartridge can be fetched from the source its manifest names, pinned in a lockfile and run only after explicit approval"
status: open
level: 10
estimate: 2d
needs:
  - "[[a-cartridge-declares-what-it-needs]]"
---

# a-cartridge-installs-from-its-source

## Outcome

A cartridge that is not in the tree can be brought in from the repository its
manifest names: fetched into a cache, pinned to an exact commit recorded in a
lockfile beside the profile, and referenced by a profile entry like any local
folder. What loads changes only when the pin changes. Nothing fetched executes —
not even its declaration read — until the user has approved that cartridge at
that pin, and an approval is for a pin, not for a repository.

Scope is retrieval, pinning and approval. Publishing, versioning across a
registry and automatic updates are not part of this outcome.

## Check

- [ ] Adding a cartridge from a local `file://` git repository writes a lockfile
      entry with its resolved commit, and a second add is a no-op that reports
      the existing pin.
- [ ] A profile entry pointing at the cache loads the fetched cartridge and
      reaches its service, with its needs resolved by the same index as a local
      folder.
- [ ] New upstream commits change nothing about what loads until the pin is
      updated explicitly; the test pushes to the source repository and asserts
      the running generation is unchanged.
- [ ] An unapproved cartridge in the cache is never executed: no process spawn,
      no Lua evaluation, and the run reports what needs approving.
- [ ] A fetch whose content does not match the pinned commit fails the install
      and leaves the cache unchanged.
- [ ] `just check` and `just test` pass.

## Approach

Observed 2026-09-12. `cartridge.json` already carries `source` as a retrieval
reference (`core/loader.rs:73`), and `sdk::Host::landscape` reports it as the
place an installed artifact came from — but nothing reads it: there is no fetch,
no pin, no cache, no lockfile. Profile paths resolve to folders under the
workspace or beside the executable, so a third-party cartridge today means
copying a folder in by hand. `.zirkle/credentials` shows the profile directory is
already where per-profile state lives, so a lockfile belongs beside `init.lua`.

## Spec

Retrieval is a cartridge, not core. Core's part is only that a profile path may
resolve into the cartridge cache; fetching, pinning and approval live in an
install cartridge, so a host that never installs anything carries none of it and
the security surface stays outside the composition core.

The pin is the contract: `<profile>/cartridges.lock` maps a cartridge name to
its source URL and resolved commit, and the cache stores that commit's tree
under a content-addressed path. Resolution reads the lock, never the network; an
install or an explicit update writes it. This is what makes a dependency tree of
linked repositories reproducible rather than "whatever upstream is today".

Trust is explicit and narrow, decided here rather than left open: approval is
recorded per cartridge and pin, the approval record is the profile's, and an
unapproved entry is inert — the composer treats it as undeclared, so it cannot
be started by demand activation either. No auto-update, no transitive approval:
a cartridge that pulls in others surfaces each of them for approval by name.
