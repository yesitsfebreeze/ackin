---
kind: principle
description: Apply when integrating a new requirement into an existing design. Redesign as if the requirement had been a foundational assumption from day one, instead of bolting it on.
group: core
---

# Redesign from first principles

When integrating a change, do not bolt it onto the existing design. Redesign
as if the requirement had been there from the start.

- Read all affected files and understand the current design.
- Ask: writing this from scratch with the new requirement, what would we build?
- Propagate the change through every reference — types, docs, examples,
  rationale sections.
- Think about the whole redesign, then deliver it incrementally.

This is the method for preserving option value when integrating changes into
an existing design. It rebuilds a design around a new requirement;
[[attack-the-premise]] questions a fact the current design assumes.
