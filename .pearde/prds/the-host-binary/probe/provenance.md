# The upstream tree is not a stable reference, and stopped being one mid-pass

`~/dev/sys/core/` is where `src/` was copied from. Three of this PRD's
acceptance boxes were written to `diff` against it. That was reasonable when
they were written and it is not reasonable now.

**What happened.** At 16:49 on 2026-09-12 the implementer ran
`diff ~/dev/sys/core/runtime.rs src/runtime.rs` and got exactly what the box
predicted: 7 renamed lines and the 10-line `Ctx::memory` deletion, nothing else.
At 17:16:45 the same file was modified by whoever is working in that repo, and
by 17:2x the same `diff` carried extra hunks — a `tokio::sync::mpsc` import and
a `pub(crate) queue: mpsc::UnboundedSender<(Value, Uid)>` field with a
three-line doc comment about emit ordering. None of that is this tree's. The box
went red without a line of this repo changing.

**Why it cannot be repaired by pointing at a commit.** `~/dev/sys` is a git
repo, but the copy was not taken from a commit. Diffing our `src/runtime.rs`
against every one of the last 25 commits that touched `core/runtime.rs` gives
669 differing lines in each case, against 17 for the working tree at 16:49. The
copy came from that repo's **uncommitted working tree**, which is still being
worked in. There is no frozen upstream object to point a box at.

**The ruling.** An acceptance box on this board may not read another project's
working tree. A box whose verdict depends on a file this repo neither owns nor
versions is not a check; it is a race. This is also the direct consequence of
the fork the user answered on 2026-09-12: *"Its own history — this project
starts tracking its own changes now, independent of whatever it was copied out
of."* A box that fails when the parent project moves is that independence not
being real yet.

**What replaces it.** This repo's own baseline commit `62fbd07`, which froze the
copy as it stood. The property worth guaranteeing is that trimming was deletion
and renaming, never redesign — and that is checkable against our own history:

```sh
git diff 62fbd07 -- src/runtime.rs      # must be empty: no pass has redesigned it
git diff 62fbd07 -- src/fiber.rs        # must be empty
```

The copy-to-baseline delta itself — the 7 renames at L59, 71, 122, 150, 477,
579, 583 and the `Ctx::memory` deletion at L387-396 — was verified against the
upstream tree while that tree still stood still, at 16:49, and its output is in
this PRD's report. It is historical evidence with a timestamp on it, which is
the most an external moving tree can ever give.

`fiber.rs` upstream has not been touched since 2026-09-11 19:08 and still
matches byte for byte, and `Cargo.toml` upstream was last touched at 13:53 and
its normalised diff is still empty. Both boxes passed on this run. They are
repointed at the baseline anyway, for the same reason: a box that passes today
because nobody happened to save a file is not a check either.
