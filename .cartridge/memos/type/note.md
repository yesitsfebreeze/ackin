---
kind: type
type: note
description: Useful facts, observations, and context worth retaining
uses:
  - usage: "[[read-usage]]"
    when: [preserve useful facts and observations]
  - usage: "[[compose-usage]]"
    when: [writing a note memo, preserve useful facts and observations]
---

# Note

One note preserves one coherent subject: a finding, explanation, reference,
or idea. Use `kind: note` and a nonempty `description`. No status is required.
Add a date when the observation can go stale, and cite the source or command
when the claim depends on evidence.

Use a descriptive title and the shortest body that preserves the useful
context. Distinguish observed facts from inference and untested ideas. Record
limitations that affect reuse. Update stale facts with fresh evidence; retain
old observations only when their history matters.

A note does not assign work or establish a decision. When an idea becomes a
commitment, create work and link it. When a choice is settled, create a decision.
When a missing answer blocks progress, create a question.

## Starting shape

```markdown
---
kind: note
description: The terminal client currently submits one line at a time
---

The input reader in `builtin/tui/cartridge.rs` emits a submission on each newline.
A multiline editor would need an explicit submit action before emitting input.
```
