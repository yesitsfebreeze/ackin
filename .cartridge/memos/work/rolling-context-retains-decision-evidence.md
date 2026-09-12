---
kind: work
description: "Compaction quality is measured across repeated summaries"
status: done
owner: "sys-opus-2026-09-12/implementer-rolling-context-retains-decision-evidence"
level: 10
priority: P1
estimate: 2h
---

# Compaction quality is measured across repeated summaries

## Outcome

A small evaluation corpus makes the intended working memory observable: goal and why, corrections, decisions with evidence, important paths with their effects, confirmed actions, uncertainties and next steps. It detects semantic loss across repeated compactions, separately from storage and byte-budget correctness.

## Spec

Files:

- `builtin/harness/eval/corpus/journal-v{1,2,3}.jsonl` — a versioned conversation in harness journal format, three cumulative snapshots (each a prefix extension of the previous): round 1 carries the corrected goal, the parked settings/page.tsx N+1 finding and the Safari 14 / legacy/export.py constraints; round 2 the TypeError failure on an empty selection later fixed by a guard, the staging-only migration permission and its confirmed 12-row result, and the batched-export decision with its evidence; round 3 the staging deploy, the still-blocked production rollout and the next regression test.
- `builtin/harness/eval/corpus/facts.json` — the rubric: `retain` facts (needles any faithful summary must keep) and `forbid` facts classified `obsolete_conclusion`, `invented_completion`, `invented_permission`; `summary_max_bytes` 6144 as in the harness config.
- `builtin/harness/eval/corpus/summaries/` — three fixed known-good working memories (rounds 1-3) and four fixed known-bad ones (lost constraint, obsolete conclusion, invented completion, invented permission) with `manifest.json` labelling the defect each must trigger.
- `builtin/harness/eval/eval_compaction.py` — stdlib-only evaluator. `--check` is the offline gate; `--model` is the opt-in live run over `ZIRKLE_EVAL_BASE_URL` / `ZIRKLE_EVAL_API_KEY` / `ZIRKLE_EVAL_MODEL` (OpenAI-compatible), three sequential compactions with the real `builtin/harness/summarize.md` prompt, writing `eval/results/<model>-<prompt-revision>.json` with model identity, prompt revision (sha256 of summarize.md) and per-round summaries, sizes and defects.
- `justfile` — `test` runs the offline gate as its last step.
- `builtin/harness/README.md` — documents corpus, rubric, gate and opt-in invocation.
- `.gitignore` — `builtin/harness/eval/results/` (generated reports) is ignored.

Steps:

1. `python3 builtin/harness/eval/eval_compaction.py --check` — validates the corpus (JSONL, tool-group integrity, prefix chaining across the three versions, rubric class coverage), scores the seven fixed summaries (known-good must be clean and within 6144 bytes; each known-bad must report exactly its labelled category), then selftests the live path against a local fake endpoint and asserts the report records model identity, prompt revision and per-round results. No network, no credentials.
2. `just test` — the full gate including step 1.

Opt-in model evaluation (never part of a gate): with `ZIRKLE_EVAL_BASE_URL`, `ZIRKLE_EVAL_API_KEY` and `ZIRKLE_EVAL_MODEL` set, run `python3 builtin/harness/eval/eval_compaction.py --model`; read the report, needle scoring is a floor, not full semantic equivalence.

Probe (commit e4f3569 on lane `work/rolling-context-retains-decision-evidence`): corpus, rubric, fixed summaries, evaluator with selftest, justfile wiring and README written; `--check` passes end to end. Left: run `just test` / `just all` on the lane and tick the boxes.

## Check

- [x] Versioned conversations cover a corrected goal, a failed action later fixed, an unresolved blocker, path-to-finding-to-effect links and an older still-relevant constraint across at least three compactions.
- [x] An explicit rubric reports lost facts, invented completion/permission and obsolete conclusions alongside size; fixed fake summaries are not presented as model-quality evidence.
- [x] The evaluator has a documented opt-in model/provider invocation and records model identity, prompt revision and results. Ordinary offline gates validate the corpus and score known-good/known-bad examples without network credentials.

```sh
python3 builtin/harness/eval/eval_compaction.py --check
just test
```

## Result

Delivered on lane `work/rolling-context-retains-decision-evidence`, commit `e4f3569`: the three-version journal corpus, `facts.json` rubric, seven fixed known-good/known-bad summaries with `manifest.json`, the stdlib-only `builtin/harness/eval/eval_compaction.py` (`--check` offline gate, `--model` opt-in live run), the `justfile` wiring, README documentation and the `.gitignore` entry for generated reports.

Gates re-run on the lane 2026-09-12, all green on the current binary:

- `python3 builtin/harness/eval/eval_compaction.py --check` — `corpus: 3 versions, 50 messages, valid`; `good-round-1..3.md ... defects=none`; `bad-lost-constraint.md: defects=['lost_fact']`, `bad-obsolete-conclusion.md: defects=['obsolete_conclusion']`, `bad-invented-completion.md: defects=['invented_completion']`, `bad-invented-permission.md: defects=['invented_permission']`; `selftest passed: live path records model, prompt revision and results`; `offline gate passed (fixture scoring only, not model-quality evidence)`.
- `just check` — passed (cargo fmt/clippy/check across the workspace, `bun install --frozen-lockfile`, `tsc --noEmit`), no warnings, exit 0.
- `just test` — passed, exit 0; the offline compaction gate runs as its last step with the output above.

The three `## Check` boxes were re-confirmed against this run; the output matches the recorded evidence verbatim, so no evidence line needed correcting. Landing is the coordinator's.

## Approach

Observed 2026-09-12: `builtin/harness/summarize.md` already asks for this behavior. `builtin/harness/tests/working.rs` supplies a fixed fake summary and proves refresh/reload/failure mechanics. Those tests cannot establish semantic summarization quality.

Audit: [[runtime-audit-2026-09-12]].

Check 1 evidence (2026-09-12, lane commit e4f3569): `python3 builtin/harness/eval/eval_compaction.py --check` reports `corpus: 3 versions, 50 messages, valid` (prefix chaining across journal-v1..v3 validated). facts.json carries the corrected goal (`goal-corrected`), the failed action fixed by a guard (`action-failed-then-fixed`: TypeError, empty selection, guard), the unresolved blocker (`blocker-production-review`), the path-to-finding-to-effect link (`path-finding-effect`: settings/page.tsx, N+1) and the older still-relevant constraints (`constraint-safari` exercised again in round 3, `constraint-legacy`), across three compactions.

Check 2 evidence (2026-09-12, lane commit e4f3569): facts.json classifies forbid facts as obsolete_conclusion, invented_completion, invented_permission plus retain needles and summary_max_bytes 6144; `--check` scores all seven fixed summaries — `good-round-1..3.md: ... defects=none`, `bad-lost-constraint.md: defects=['lost_fact']`, `bad-obsolete-conclusion.md: defects=['obsolete_conclusion']`, `bad-invented-completion.md: defects=['invented_completion']`, `bad-invented-permission.md: defects=['invented_permission']` — each known-bad reporting exactly its labelled category, and the gate prints `offline gate passed (fixture scoring only, not model-quality evidence)`.

Check 3 evidence (2026-09-12, lane commit e4f3569): `builtin/harness/README.md` documents the opt-in invocation over `ZIRKLE_EVAL_BASE_URL` / `ZIRKLE_EVAL_API_KEY` / `ZIRKLE_EVAL_MODEL` with `python3 builtin/harness/eval/eval_compaction.py --model`; the `--check` selftest against a local fake endpoint asserts the report records identity and prints `selftest passed: live path records model, prompt revision and results`; the offline gate validates the corpus and scores known-good/known-bad fixtures with no network and no credentials (`offline gate passed (fixture scoring only, not model-quality evidence)`).
