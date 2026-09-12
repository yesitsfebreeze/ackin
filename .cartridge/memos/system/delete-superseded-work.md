---
kind: system
description: "No legacy retention: superseded code, docs and fixtures are deleted in the same change; git is the archive"
order: 5
uses:
  - usage: "[[read-usage]]"
    when: ["ending work on any change that replaces or abandons an older design"]
---

# Delete superseded work when you move on

When a design moves forward, the old one is deleted in the same change: dead
code, replaced modules, stale fixtures, obsolete docs, vendored leftovers,
empty folders. Git history is the archive — nothing is kept in the tree "for
reference" or "just in case". A finished change leaves no fossils behind it.
Only frontier work (active designs and in-progress work memos) stays in the
tree. The decision record is the one exception: decisions may be marked
superseded, because that trail is how the record stays honest. If a stale
`work` memo describes a shipped or abandoned design, delete it rather than
leaving it marked done. When you spot cruft you did not create, delete it and
say so.
