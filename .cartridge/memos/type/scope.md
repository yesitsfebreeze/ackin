---
kind: type
type: scope
description: A feature entry point and the references that explain it
---

# Scope

Use `kind: scope`, description, `entry` (a memo reference), `members` (a nonempty
list of memo references), and optional `uses` per [[usage]]. Entry must occur in
members. References may be canonical memo paths or wiki links. Cycles are refused.
A resolved scope returns its entry; read the scope to inspect further members.
It is a feature map, not an instruction to load every body into context.
