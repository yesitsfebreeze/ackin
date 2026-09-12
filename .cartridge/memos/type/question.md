---
kind: type
type: question
description: An unresolved choice or missing answer that needs an explicit resolution
uses:
  - usage: "[[read-usage]]"
    when: [record an unresolved choice]
  - usage: "[[compose-usage]]"
    when: [writing a question memo, record an unresolved choice]
---

# Question

One question memo captures one answer needed to proceed. State the uncertainty
plainly and explain what depends on it. Investigate facts that can be checked;
ask a person when their intent, preference, or authority determines the answer.
Do not turn every implementation detail into an approval request.

## Fields

Every instance has `kind: question`, a nonempty `description`, and `status`:
`open`, `answered`, or `cancelled`. Optional `asked_of` names the person or role
whose answer is needed. Work that cannot proceed links here through `needs`.
A cancelled question does not satisfy that dependency: update the dependent
work's scope or prerequisite explicitly.

## Body

- `## Question`: the specific question and why its answer matters.
- `## Options`: when there is a choice, give the viable alternatives and their
  consequences; label a recommendation and explain it. Omit for a factual question.
- `## Answer`: required when answered; record the answer, its source, and date.
  Quote a person's answer accurately and separate it from your interpretation.
- `## Resolution`: use when cancelled to explain why no answer is needed.

Keep an open question open until evidence or an actual answer resolves it.
Elapsed time is not an answer. If the answer establishes a lasting rule, write a
separate decision and link to it from Answer; retain this question as the
record of the uncertainty. Create work for any resulting implementation.

## Starting shape

```markdown
---
kind: question
description: Determine whether drafts survive session closure
status: open
---

## Question

Should explicitly closing a session retain its unsent draft for reopening?
The answer determines when draft storage is deleted.

## Options

- Retain it: reopening recovers the draft; storage persists until deletion.
- Discard it: closing clears the draft; accidental closure loses it.

Recommendation: retain it until the session is explicitly deleted.
```

The cartridge stores the answer and status; it does not decide that a question
has been resolved or unblock work automatically.
