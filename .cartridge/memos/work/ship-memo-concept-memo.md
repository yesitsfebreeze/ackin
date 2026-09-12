---
kind: work
description: Ship the memo-concept memo with the default record
status: done
owner: claude
uses:
  - usage: "[[read-usage]]"
    when: ["understanding what was shipped to define the memo concept, verifying the memo memo"]
---

## Outcome

Every zirkle install's record answers "what is a memo" in its own words (not only
the protocol): a shipped `kind: note` at `@memo/note/memo.md` explaining the
record, kinds, composition, and the memo-over-code rule.

## Spec

The note is `kind: note`, shipped under `builtin/memo/.zirkle/memos/note/memo.md`,
with `uses` matching record and memo-vs-code questions. Content: a memo is a
Markdown file with frontmatter in `.zirkle/memos/`, kinds declared by `kind: type`
memos, system composition with `enabled: false` opt-outs, shipped records merged
as `@cartridge/...` with workspace shadowing, resolver/usage discovery, and the
rule that anything expressible as a memo is a memo — code only where enforcement
or execution requires it. Links `[[type]]`, `[[system]]`, `[[resolver]]`,
`[[corrections]]`.

## Check

- [x] `builtin/memo/.zirkle/memos/note/memo.md` ships with the memo cartridge record.
- [x] Explains the record and kinds without duplicating the type/type.md protocol.
- [x] States the memo-over-code rule from the shipping-defaults discussion.
- [x] Linking valid in the merged record ([[type]], [[system]], [[resolver]], [[corrections]] resolve).

## Approach

1. Author the shipped note as the concept companion to the protocol type.
2. Wire [[memo]] into `@memo/note/program-map.md`.
3. Verify shipped record semantics (`cargo test -p memo`) and resolve.

## Result

Landed 2026-09-12. `builtin/memo/.zirkle/memos/note/memo.md` authored and wired
into program-map. Verified: `cargo test -p memo_cartridge` 29/29; real-profile
composition 11/11; real-host resolve ranks `@memo/note/memo.md` first for a
memo-vs-code query.
