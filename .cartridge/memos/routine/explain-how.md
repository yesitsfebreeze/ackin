---
kind: routine
description: Answer "how does X work" — explore the subsystem, build the mental model a senior engineer would onboard with, and hand back an explanation, not annotated source.
uses:
  - usage: "[[run-usage]]"
    when: [asking how a subsystem works, walking code before changing it, deciding where something should live or which layer owns it]
---

Companion to [[explain-why]]. This answers what the code does and how it
works; that answers what forces led to its shape. [[blast-radius]] answers
what it breaks somewhere else.

1. Assess complexity

If the scope is ambiguous, state your interpretation and explore. The caller
can redirect.

- **Simple** — one module, a small utility, a narrow question. No explorers.
  One pass explores and explains. Go to step 3.
- **Complex** — a subsystem spanning several files or cartridges, a
  cross-cutting feature, a full architectural overview. Fan out first. Go to
  step 2.

When in doubt, take the simple path.

2. Explore (complex only)

Decompose the question into two to four exploration angles, each a distinct
slice. Dispatch all of them in one message, read-only. Each gets the same
brief: its angle, the entry points it should start from, and the instruction
to report file paths and line numbers, not prose summaries.

Bulk output stays with the explorers, per [[guard-the-context-window]].

3. Explain

One pass turns the findings into one explanation — for a complex question a
separate synthesis pass over the explorers' findings, for a simple one the
single pass that also explored.

Target the level of a senior engineer onboarding onto the subsystem: enough to
build a working mental model, not so much that it reads like annotated source.

4. Present

Sections, dropping any that do not apply:

- **Overview** — what this subsystem is for, in two or three sentences.
- **Key concepts** — the nouns the rest of the explanation needs.
- **How it works** — the runtime flow, in order, with `file:line` anchors.
- **Where things live** — the map from concept to file.
- **Gotchas** — what a newcomer gets wrong.

Light edits for clarity are fine. Do not substantially rewrite the
explanation. Run it through [[unslop]] before it goes out.

Check: every claim in the explanation names a real `file:line`. A claim that
cannot be anchored is an inference and is labelled as one.

Failure: the explorers disagree about the flow. Do not average them. Read the
contested path yourself and say which one the code supports.
