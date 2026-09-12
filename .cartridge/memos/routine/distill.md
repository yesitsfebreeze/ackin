---
kind: routine
description: One distillation pass over the record — take the memos written since the last pass, rate each on a fixed rubric, fold the duplicates into one stronger claim, and delete what rates below the bar.
uses:
  - usage: "[[run-usage]]"
    when: [the record has grown faster than it is read, memos repeat each other, asking which memos are worth keeping, condensing a kind folder, a distill nudge fired]
---

Sibling of [[hygiene]]. Hygiene finds the one memo that has rotted and repairs
it; distillation takes a whole cluster and makes it smaller without making it
say less. [[improve]] adds what the record does not hold; [[quality]] removes
what the code does not need; distill removes volume, never claims.

Text in, fewer and stronger statements out. The record is the text here, so
every fold must carry the losing memo's evidence — a number, a path, a symbol,
a command — into the survivor. A fold that loses evidence is the one failure
this routine exists to prevent.

## 1. Read

- `type/type.md` and the `type` memo of the kind being distilled. A fold never
  crosses kinds: two claims of different kinds are not duplicates however alike
  they read.
- `git status --short .cartridge/memos`. An uncommitted memo has no archive, so it
  is never folded or deleted this pass — commit it first or skip it. A memo
  another session is mid-edit on is not this pass's candidate either.
- Records shipped by enabled cartridges merge in read-only. Only
  `.cartridge/memos/` is writable; a shipped memo is never a fold target.

## 2. Size the record

```
find .cartridge/memos -name '*.md' | wc -l
cat .cartridge/memos/*/*.md | wc -w
for d in .cartridge/memos/*/; do echo "$(ls $d | wc -l) $d"; done | sort -rn
```

Measured 2026-09-12: 435 memos, 193,520 words, fattest folders `routine/` 167,
`note/` 123, `work/` 68. Record the two totals; step 7 compares against them.

## 3. Pick the batch

**The watermark is a commit subject, not a file.** Every pass commits as
`distill: <cluster>` (step 6), so the last pass is a `git log` away and the
batch is what the record gained since:

```
mark=$(git log -1 --format=%H --grep='^distill:')
git diff --name-only "$mark..HEAD" -- .cartridge/memos
```

No `distill:` commit yet means the whole record is the batch. Untracked memos
are outside it by construction — step 1 refuses them, and the diff cannot see
them.

**Then one cluster inside the batch.** Never the whole batch at once, never
more than twenty memos. A cluster is one kind folder plus one filename prefix:

```
cd .cartridge/memos/note && ls *.md | sed 's/-.*//' | sort | uniq -c | sort -rn
wc -w *.md | sort -rn | head
```

The prefix is a topic by convention, not by rule — read the descriptions before
trusting it. Cap a cluster at roughly twenty memos or ten thousand words, which
is one agent's honest read. Prefer the cluster whose word count is highest per
claim: that is where the duplication is.

The cheap hit is a same-stem pair, and the probe above finds it —
`note/work-checkpoint.md` and `note/work-checkpoint-2026-09-12.md`, 1,763 and
1,813 words, are one claim in two files.

## 4. Rate — the agent run

One agent per cluster, dispatched in a single message when several clusters are
in the batch. Each agent reads every memo of its cluster **in full** and returns
a table. It writes nothing; rating and landing are separate so a bad rating
costs a read and not a revert.

The prompt gives the agent the cluster's file list and this rubric. Three axes,
0 to 2 each:

- **Load** — would a reader act differently for having read it? 0 restates what
  the code, another memo, or `CLAUDE.md` already says.
- **Proof** — does it carry a command, path, symbol or number a reader can
  check? 0 is unfalsifiable prose.
- **Reach** — is it linked from another memo, and does its `description` name
  the situation that should surface it? 0 is unreachable and unfindable.

Score 0 to 6:

- **5–6** keep as written.
- **3–4** fold into a sibling, its evidence carried over.
- **0–2** delete. Git is the archive.

The agent returns one line per memo — `<leaf> <load>/<proof>/<reach> = <score>
— <one clause of why>` — then a fold plan of `<surviving leaf> <- <losing
leaf>, <losing leaf>`, each losing leaf followed by the exact evidence it
contributes to the survivor. No prose beyond that.

## 5. Verify before landing

The rating is a proposal; this step is the gate.

- Refuse any fold whose plan names no evidence from a loser — that is a
  deletion wearing a merge's clothes, and if deletion is right it is rated 0–2
  and deleted as one.
- Refuse a delete on a memo something still links. Check each losing leaf:

```
rg -o "\[\[<leaf>" .cartridge/memos -g '!*/<leaf>.md'
```

  Every hit is a link this pass repoints in the same change.
- Spot-check one 5–6 and one 0–2 against the file yourself. An agent that
  rated a memo it did not read shows up here.

## 6. Land

- Write the survivor first, through `memo(op="write", ...)`, with the losing
  evidence already in its body. Read its warnings and saved status.
- Repoint every inbound link found in step 5, through managed writes.
- Then `rm` the folded and deleted files. There is no delete op; a raw `rm`
  bypasses the index refresh, so follow the batch with one more managed write
  of a surviving memo and confirm the derived index no longer names the
  removed leaves (`memo(op="index")`).
- One commit per cluster, subject `distill: <cluster>`. The prefix is load
  bearing: it is the watermark step 3 reads and the thing the nudge counts
  from. A pass that lands under any other subject is invisible to the next one.

## 7. Report and loop

One line: the cluster, before and after counts and words, and what folded into
what. A judgment the next pass should not re-argue — a near-duplicate
deliberately kept apart, a low-scoring memo kept for a reason the rubric cannot
see — becomes one `decision` memo, not a comment.

Back to step 3 with the next cluster of the same batch. The loop ends when no
cluster holds two memos scoring 4 or below, or when the caller stops it.

## The nudge

The harness asks for this pass on its own. The memo cartridge owns the record,
so it is the one that counts the debt: `memo(op="system")` returns a `distill`
line beside the composed template (`builtin/memo/src/record.rs`,
`distill_due`), and the harness appends it below the instruction frame as the
`Record upkeep` block, the same way it appends run telemetry
(`builtin/harness/main.rs`, `build_system`). Empty debt, no block.

It counts memo files changed since the `distill:` watermark, not turns — a long
session that writes nothing to the record owes no pass — and stays silent below
`CARTRIDGE_DISTILL_EVERY`, default 40. The block offers; the user answers. Nothing
in the harness starts this routine.

## Check

The after count and word count from step 2 are both lower, and every claim that
scored 3 or above is still readable somewhere in the record. `memo(op="index")`
names no leaf that was removed, and `rg '\[\[' .cartridge/memos` resolves — no link
points at a folded leaf. `git log -1 --format=%s --grep='^distill:'` returns
this pass.

## Failure

A fold that dropped a number, path or command: the survivor is wrong and the
loser is gone. `git checkout .cartridge/memos` before the commit, `git revert`
after, then re-fold with the evidence named explicitly in the plan. This is why
step 1 refuses uncommitted memos and step 6 commits per cluster.

An agent returning a rating with no per-memo clause has summarized the folder
instead of reading it. Discard the table and re-dispatch with a smaller
cluster; do not repair it by hand.
