# Drill — 3 forks over the open frontier

## Q1 the-host-binary Whether anything built here can be checked

A system process on this machine has been stuck at full load since yesterday, and
while it is, nothing newly built will start — so no check that compiles this
project can run at all. Work can keep moving without that signal, but nothing it
produces can be shown to actually work?

- Restart the machine — the stuck process clears, and every check that has been failing can finally run. (recommended)
- Carry on without the signal — work keeps moving, and nothing is proven to build until the machine is dealt with.
- Build elsewhere — the checks move to another machine or a container, and this one is only used for editing.

## Q2 the-host-binary Where this project's history lives

Nothing here is under version control, so there is no way to undo a change or to
see what an earlier run did. The work about to start rewrites files in place, and
this settles whether any of it can be taken back?

- Its own history — this project starts tracking its own changes now, independent of whatever it was copied out of. (recommended)
- Share the parent project's history — it moves in beside the code it was copied from and inherits that project's setup.
- No history — nothing is tracked, changes cannot be undone, and each run overwrites whatever the last one left.

## Q3 the-manifest What an inner cartridge offers the outside

When one capability is built out of others, you are choosing whether the things
those inner pieces offer are visible to everything outside, or hidden until the
outer one chooses to pass them on. Visible everywhere means fewer steps and more
name clashes; hidden means real nesting and more wiring?

- Hidden until passed on — each piece names what it offers outward, so names never clash and nesting is real. (recommended)
- Visible everywhere — whatever an inner piece offers, the outer one offers too, with no wiring and possible name clashes.
- Visible unless hidden — everything passes outward by default, and a piece can name the few things it keeps to itself.
