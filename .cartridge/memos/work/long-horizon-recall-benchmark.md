---
kind: work
description: Score the compaction, ring and working-memory machinery against LongMemEval-style recall tasks
status: open
level: 10
---

## Outcome

An offline eval measures what the harness's long-horizon machinery actually
retains: run the compaction, turn ring and working-memory pipeline over
multi-session corpora and score recall, temporal reasoning, knowledge updates
and abstention, in the shape of LongMemEval (arXiv 2410.10813). The existing
`eval_compaction.py` scores fixed summaries; nothing measures whether a
recalled turn answers a question about it.

## Check

- [ ] An offline eval accepts a corpus of sessions with questions whose
      answers live in older turns, distills them through the real ring
      sweep, and scores answerable recall against the memory bank.
- [ ] It reports per ability (extraction, multi-session, temporal,
      knowledge-update, abstention) so regressions name the degraded skill.
- [ ] A baseline number is recorded so future harness changes are measured
      against it, not asserted.

## Context

LongMemEval's headline finding: commercial assistants and long-context models
drop about 30% accuracy across sustained interaction — exactly the failure
mode the ring/compaction design exists to prevent, currently unmeasured here.
Peer designs (Mem0) cite LoCoMo/LongMemEval/BEAM as their evidence; the
harness should cite numbers, not architecture.