---
kind: type
type: resource
description: Usage declarations for one existing workspace file
---

# Resource

Use `kind: resource`, description, one literal workspace-relative `target`, and
`uses` as defined by [[usage]]. Keep the original file in place. One resource per
target; add further usages to that memo. Absolute paths, traversal, symlinks,
`.cartridge/memos/` targets, and Git metadata are refused. Native memos declare uses directly.
A resource may describe a binary; the resolver returns metadata, never executes it.
Missing targets remain readable for repair and appear in coverage diagnostics.
