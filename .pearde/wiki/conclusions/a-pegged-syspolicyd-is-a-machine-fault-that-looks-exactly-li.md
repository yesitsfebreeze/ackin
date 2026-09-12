---
title: a-pegged-syspolicyd-is-a-machine-fault-that-looks-exactly-li
date: 2026-09-12
type: conclusion
tags: [build-signal, conclusion, macos, toolchain]
sources:
  - "[[260912-ebde]]"
  - "[[260912-fb23]]"
  - "[[260912-7a41]]"
derived_from: []
---

# A pegged syspolicyd is a machine fault that looks exactly like a broken build

Three separate investigations on this board reached the same place from
different directions, and the conclusion is worth stating once so a fourth does
not start.

**The fault.** On macOS 25.2 / Apple silicon, `/usr/libexec/syspolicyd` can get
stuck at ~100% CPU and stay there across days and across logins. While it is,
dyld blocks forever on the code-signing verdict it is no longer answering, so
**every freshly linked Mach-O hangs in `_dyld_start` before reaching `main`**.

**Why it reads as a build failure and is not one.** `rustc` still runs — its
own verdict was cached long ago — so compiling a single file appears healthy.
What stalls is anything the build must *execute*: `build.rs` scripts and
proc-macro dylibs. `cargo build`, `cargo check` and `cargo test` therefore do
not error, they **hang at 0% CPU** after roughly ten crates, and each attempt
strands another `cargo` process. An agent reading only "the build is not
finishing" will spend its whole window on a crate that is fine.

**The test that separates the two,** and the only build-shaped command worth
running before it passes:

    d=$(mktemp -d); printf 'fn main(){println!("hi");}\n' > "$d/h.rs"
    rustc -o "$d/h" "$d/h.rs" || exit 1
    perl -e 'alarm 20; exec($ARGV[0])' "$d/h"   # 0 healthy, 142 hung in dyld

**What does not clear it.** Waiting, `xattr -c`, re-signing with `codesign
--force`, disabling quarantine on the output directory, a new terminal, a new
toolchain, `cargo clean`. Six workarounds across two passes, none of them
touching the cause.

**What does.** Restart the daemon, not the machine: `sudo killall syspolicyd`,
launchd respawns it immediately, with `sudo killall amfid` as its companion
since the two cooperate on the same verdict. A reboot also works and is not
required — which matters, because the person at the keyboard may have a session
they do not want to lose, and on this board they declined one.

**The operational consequence for an agent.** Both commands need `sudo`, so an
agent cannot clear this itself. It has to hand the user the command and then
**carry on without the signal**: do the editing work, tick only the boxes that
are a grep, a diff or a file-structure fact, leave every compile and run box
open, and say plainly that nothing was verified. A report claiming a build it
did not get is worse than an open box, because the next worker trusts it.
