---
kind: type
type: type
description: Declares a memo kind, including this declaration kind
uses:
  - usage: "[[read-usage]]"
    when: [understanding memo kinds and declarations]
  - usage: "[[compose-usage]]"
    when: [writing a type memo, understanding memo kinds and declarations]
---

# Types and memos

The memo cartridge owns this record in `.cartridge/memos/`. A memo is a UTF-8 Markdown
file with YAML frontmatter and a body. Every memo has a `kind` naming a
declared type and a nonempty `description`. Its path is `<kind>/<name>.md`;
a memo shipped by a cartridge appears as `@<cartridge>/<kind>/<name>.md`.
Leaf names are unique across the record; `[[name]]` links resolve by leaf, and
a workspace memo shadows a shipped memo of the same leaf.

A memo with `kind: type` declares the kind named by its `type` field.
This memo declares `type` itself. A declaration's body explains the kind's
purpose, its fields, and how to write and use its memos. Declare a kind before
writing its instances. Changing a type requires editing its memo, not code.

Use the memo tool to list, read, and write memos. `types` returns declarations
with their instructions; `list` can filter by kind. Writes validate the record
before atomically replacing a file. The `index` operation derives an overview
from current memos; there are no hand-maintained index files. Structural rules
are enforced by the cartridge; prose in declarations guides the reader.

The cartridge seeds a new record with type declarations, usage definitions, and
system prompt memos. [[usage]] defines the shared usage metadata. Existing records are never reset or merged with the seeds. Read [[system]]
to understand how the harness builds its prompt.
