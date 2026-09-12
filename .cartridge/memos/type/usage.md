---
kind: type
type: usage
description: An intended action on a memo or file, separate from its kind
---

# Usage

A usage memo has `kind: usage`, a unique lowercase `name`, a description, and
instructions explaining the action and its evidence. Read, edit, run, and compose
are supplied; a new name is declared here, not inferred from a file extension.
The declaration makes an action discoverable, not executable or authorized.

Any memo may declare `uses`: a list of `usage` wiki links, each with nonempty
`when` phrases and optional `not_when` and `tags` lists. One entry per usage;
multiple situations belong in that entry. The service validates shapes and links.
No uses means undeclared coverage; ordinary list/read still work. No inheritance
from type to instance. Resolve returns ranked references and revision digests.
