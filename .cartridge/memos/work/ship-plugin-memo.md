---
kind: work
description: Ship the plugin-concept memo with the default record
status: done
owner: claude
uses:
  - usage: "[[read-usage]]"
    when: ["understanding what was shipped to define the plugin concept, verifying the plugin memo"]
---

## Outcome

Every zirkle install's record answers "what is a plugin" without the plugin→cartridge
rename hiding the answer: a shipped `kind: note` at `@memo/note/plugin.md`
defining cartridge, service and the surviving senses of the word plugin.

## Spec

The note is `kind: note`, shipped under `builtin/memo/.zirkle/memos/note/plugin.md`,
with `uses` matching extension-model and plugin-reference questions. Content: the
extension unit is a cartridge (`cartridge.json` + relative Lua entry returning a
component table or `zirkle.process` descriptor); a service is a provided key others
inject, enforced at runtime; the word plugin is legacy, recorded in
`[[plugin-folder-manifests]]`; two surviving senses are ui feature modules and
memory `kind: plugin` notes. Links `[[program-cartridge-contracts]]` and
`[[core]]`.

## Check

- [x] `builtin/memo/.zirkle/memos/note/plugin.md` ships with the memo cartridge record.
- [x] Presents the rename honestly (decision/plugin-folder-manifests.md) and names the surviving senses.
- [x] Linking valid in the merged record ([[program-cartridge-contracts]], [[core]], [[plugin-folder-manifests]] resolve).

## Approach

1. Author the shipped note as a terminology map with the facts of the extension model.
2. Wire [[plugin]] into `@memo/note/program-map.md`.
3. Verify shipped record semantics (`cargo test -p memo`) and resolve.

## Result

Landed 2026-09-12. `builtin/memo/.zirkle/memos/note/plugin.md` authored and wired
into program-map. Verified: `cargo test -p memo_cartridge` 29/29; real-profile
composition 11/11; real-host resolve ranks `@memo/note/plugin.md` second for a
plugin/core query (first is the core note).
