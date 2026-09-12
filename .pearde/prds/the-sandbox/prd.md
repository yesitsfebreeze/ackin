---
state: open
origin: requested
priority: 60
complexity: 0
blast-radius:
needs:
  - the-manifest
  - the-resolver
---

# The sandbox

Every cartridge is a child process, so every cartridge gets a policy derived
from its own manifest: `sandbox-exec` on macOS — present on this machine,
deprecated but functional, verified working — and Landlock with seccomp on
Linux.

This is the newest requirement and the one the existing system does not have at
all. It is what turns "a sandboxed tool belt of things we can vibe code" from an
intention into a property of the machine. A cartridge written carelessly is
contained by what it declared, not trusted to behave, and the person writing it
does not have to be careful for the system to stay safe.

[[the-manifest]] is the policy. There is no second file and no separate grant:
what a cartridge asked for is both what it gets and the wall it hits.

Must not change: a cartridge that declares nothing gets nothing. The empty
manifest is the tightest sandbox, not the loosest.

At the end, a cartridge reaching outside its declared capabilities is stopped by
the operating system rather than by review.
