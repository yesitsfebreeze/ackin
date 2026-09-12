---
kind: routine
description: Conduct a deep plan/design interview, persist each Q&A session, and update topic files once a coherent picture exists.
uses:
  - usage: "[[run-usage]]"
    when: [splitting work, designing anything non-trivial]
---

1. Prerequisites

- Always read `memos/SYSTEM.md` and every `kind: question` memo first, plus the `kind: decision` memos the topic touches. A new one lives flat in `memos/intake/` until `kern compact` folds it into `memos/question/` or `memos/decision/`; the rest already live there.
- Read `git diff` and `git status --short` to understand current changes before generating follow-up questions.
- If the current topic file has unanswered entries (`A: ?`), resolve those gaps before adding completely new topics.
- After the user answers, immediately replace `A: ?` with the exact answer text in the same block.

2. Topic and storage model

- Each topic is one memo, `kind: question`, stored as its own file.
- A topic is created the moment a new subject is identified that does not yet have a file.
- Determine the topic name and derive a slug (lowercase, hyphen-separated).
- Create or open the topic file at `memos/intake/<topic>.md` with `kind: question`. Create and update through `memo(op="write", path=…, body=…)`; successful managed writes refresh its index entry.
- All questions and answers for that topic live only in that file.
- Once every answer is in and the picture holds, rewrite the file through `memo` as a decision memo in place — `kind: question` becomes `kind: decision` and the file stays where it was born. Inspect the write result and warnings.

3. Interview behavior

- Interview relentlessly about every aspect of the design until a shared understanding exists. Walk down each branch of the design tree, resolving dependencies between decisions one-by-one. For each question, provide your recommended answer.
- Before asking the first set of questions for a topic, append a new question block to the topic file using exactly this structure:
- If required details are discoverable by codebase inspection, prefer inspecting code.

4. Ask style.

- Use you agent native question tool, if none is available, use this format:

```
? <clear-short-question-name>

<explanation-of-what-is-asked-and-what-it-entails>

R: <recommended-answer>
1: <best-option-to-R>
2: <best-option-to-1>
```
