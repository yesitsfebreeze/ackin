---
kind: routine
description: Answer "why is it this way" — anchor the question in code, investigate every evidence source in parallel, and return a cited read on the decisions and tradeoffs with confidence separated from inference.
uses:
  - usage: "[[run-usage]]"
    when: [asking why a design is shaped this way, investigating a regression, writing a postmortem, recovering the rationale behind a threshold]
---

Companion to [[explain-how]]. That answers what the code does; this answers
what forces led to its shape.

Operate as a careful, cautious, precise investigator. Be honest about what you
know against what you are inferring, and keep the two separated in the output.

1. Understand the target and the question

The **target** is a chunk of code, a pattern, a feature or a named design
decision. The **question** is a rationale, a tradeoff, a motivating edge case,
an external constraint, dead code, or a history sweep.

If the target is vague, make your best guess from context — open files, recent
edits, what was just discussed. State the interpretation in one line so the
caller can redirect, then proceed. Do not block on the question, per
[[never-block-on-the-human]].

2. Establish the code anchor

Before dispatching anything, anchor the investigation in concrete code: file
paths and line ranges, key symbols, an initial commit list, and PR numbers
from merge commits.

```
git blame -L <start>,<end> <file>
git log --follow -p -- <file>
git log --oneline -20 -- <file>
git log -1 --format=%B <commit>
gh pr view <number> --json title,body,author,createdAt,mergedAt,labels,closingIssuesReferences,comments,reviews
```

Build this inline and pass it to every investigator.

3. Investigate every available source in parallel

List the evidence categories this environment can actually reach, then
dispatch one investigator per category in a single message. One investigator
owns exactly one source — never ask one to cover several.

1. **Source control.** Git and `gh`. Always available, always dispatched. Best
   at implementation-time rationale captured during review.
2. **Issue tracker.** Best at the product or business forcing function.
3. **Long-form documents.** Best at design rationale written before it became
   code.
4. **Real-time chat.** Best at deliberation that never reached a doc.
   Strongest when the paper trail is thin.
5. **Infrastructure observability.** Best at the runtime reality that
   motivated the code. Strongest for timeouts, retries, rate limits, circuit
   breakers.
6. **Error tracking.** Best at the exceptions that motivated defensive code.
   Strongest for guards, catch blocks, retries.
7. **Analytics warehouse.** Best at the product and data reality behind a
   flag, an experiment or a number.

Also dispatch an incident and postmortem sweep when the target code looks
defensive: null checks, retry logic, timeout handling, rate limiting, feature
flags, memory guards.

Aim for a complete coverage map, not a minimal one. Document the null; do not
skip the search. Skip a category only with a written justification that goes
in the output: either no source of that kind exists here, which is a gap and
not a choice, or the source is provably irrelevant, which is a high bar.

4. Synthesize

One pass merges the findings, including the nulls and the justified skips,
against the code anchor and the original question. Keep the confidence
language it produces; do not rewrite it into certainty.

5. Present

- **The question.**
- **The code in question.**
- **What we found** — cited, one `file:line`, commit, PR or ticket per claim.
- **What we can reasonably infer** — labelled as inference.
- **Competing hypotheses.**
- **What we don't know.**
- **Sources consulted** — one line per investigator, including the ones that
  returned nothing and the ones skipped, with the reason.
- **Confidence summary.**

When the question precedes an actual change, convert the findings into a
Preserve / Change / Avoid / Risk constraint set for the change.

Check: every factual claim carries a real citation. A search that found
nothing is an answer and is reported as one. Never invent a caller, a PR or an
API.

Failure: recency bias. The most recent commit is not authoritative — the
current shape is usually the accretion of many earlier decisions. Trace back.
