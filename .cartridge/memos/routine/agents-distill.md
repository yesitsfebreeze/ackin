---
kind: routine
description: Tighten a memo body to a minimal, useful brief. Use when a memo reads wordy, restates its
  frontmatter, or explains the runner instead of the tool.
uses:
- usage: '[[run-usage]]'
  when:
  - tighten a memo body
  - a memo reads wordy or restates its frontmatter
  - distill a routine to a brief
  tags:
  - distill
  - lint
  - authoring
  - agent
---

## Inputs

No prerequisites beyond the commands named below. Recipe parameters are named in Do; supply paths and arguments for the current task.

## Do

**Brief.** Rewrite the body of the memo at `path` to the smallest text that is
still useful. Keep frontmatter and every fenced block verbatim; change only prose.

Keep: when to reach for the file, non-obvious gotchas, argument defaults. Cut: the
description restated, the recipe narrated, the runner or format explained, filler.

- The description is the *when* — never repeat it.
- The recipe is the *how* — never narrate what it plainly does.
- One gotcha per line; prefer a short list to paragraphs.
- If the body adds nothing past frontmatter and recipe, leave one line or none.

```just
distill path:
  body=`cat {{path}}`
  claude --print --prompt "Tighten this memo body to a minimal useful brief. Keep the frontmatter and every fenced block verbatim. $(body)"
```

## Notes

- One memo per run; loop over `.cartridge/memos/**/*.md` to sweep the record. Write the result back
  through the memo tool so the record stays validated.

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `distill` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
