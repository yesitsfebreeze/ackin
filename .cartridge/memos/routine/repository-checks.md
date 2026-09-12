---
kind: routine
description: Run the gates the change actually reaches, and the full suite only before pushing
uses:
  - usage: "[[run-usage]]"
    when: [checking repository changes before committing, verifying implementation changes]
    tags: [tests, clippy, formatting]
---

## Inputs

The workspace root and the changed files.

## Do

Scope the gates to what the change reaches, then widen once:

1. Name the crates and packages the changed files belong to, and the ones whose
   tests exercise them. `cargo test -p <crate>` for each, plus
   `cargo clippy -p <crate> --all-targets` and `cargo fmt --all -- --check`.
   A change under `builtin/ui` runs `bun run --cwd builtin/ui check` and its
   tests instead; a change to a python fixture runs that fixture.
2. A change that touches no code — a memo, a README, a justfile recipe, files
   moving in git — runs no test suite. Verify it directly: the recipe parses,
   the index holds what a fresh checkout needs, the link resolves.
3. Run `just check` and `just test` once, before pushing, not before each
   commit. That is the gate that covers the whole tree; everything before it is
   the gate that covers your diff.

## Check

Every command in the scope you named exits zero, and you can say which crates
the diff touched and why the rest are unreachable from it.

## Failure

Read the failing check, reproduce it, and repair the relevant scope. Do not mark
work done while a required check is failing. Preserve changes owned by others.
A full-suite failure that your scoped run could not have caught is worth a note:
either the scope was wrong, or the suites are coupled in a way worth recording.
