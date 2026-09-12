---
kind: routine
description: Assess the agent harness — what it has that misbehaves and what the ecosystem has that it lacks — from evidence into blocked work proposals presented as questions for explicit user approval.
uses:
  - usage: "[[run-usage]]"
    when: [checking what helps or hinders the agent harness, or whether a tool or plugin worth installing exists, before authorizing improvements]
---

# self-improve

One run produces an evidence-backed approval queue for improving the agent's
working environment. This is an assessment, not an execution loop.

1. Prepare the assessment

- Read SYSTEM (`memos/SYSTEM.md`), memo-writing, memo-atomicity,
  memo-index, [[terminal]], and system-commands. Resolve linked leaves
  with `just memo <leaf>`; use the editor and lazygit as [[terminal]] directs.
- Read [[improve]], [[work]], and [[drive]] for the research/execution boundary,
  plus work-memo, [[question]], [[decision]], and [[drill]] for the record
  shapes. Do not start those loops: this routine's approval gate governs here.
- Read [[scout]] for the ecosystem scan below. Its routes and its two-axis rule
  are not restated here; this routine calls them and files the verdict through
  its own approval gate rather than scout's.
- Use the caller's scope and supplied session evidence. Without a scope,
  assess this project's currently loaded instructions and tool interactions
  from this session only; do not claim a machine-wide audit.
- Inventory `CLAUDE.md`, `.agents/skills/`, `.claude/agents/`,
  `.claude/settings.json`, `scripts/`, and `justfile` with
  `rg --files --hidden --no-ignore .agents .claude scripts -g '!worktrees/**' -g '!.claude/worktrees/**'`
  and `just --list`. `--no-ignore` is what makes an entry point visible at all:
  `.gitignore` excludes `/.claude/*`, so without it the command returns the
  scripts and nothing else. `.agents` is named because the skills live there and
  `.claude/skills` is a symlink rg does not descend
  (one-skill-file-per-routine-under-two-harness-names). Open only the paths
  actually involved in the scoped flow; absent optional paths are not defects.
  Read configuration without copying credentials into the record. User-global
  configuration and transcripts are out of scope unless the caller identifies
  them.
- Read `memos/index.json`, then `memos/work/index.json`,
  `memos/question/index.json`, and `memos/decision/index.json`. These include
  intake entries. Read the relevant source `path` values relative to `memos/`.
  Check fresh files missed by an older index with
  `rg -n '^(kind: (work|question|decision)|status:|claim:|A:)' memos/ -g '*.md'`.
- Ask `mcp__kern__query` with `text` describing the scoped mechanism and `k: 10`
  for prior findings; if unavailable, report the limit and use the indexes.
  Read `git status --short` and `git -C memos status --short`; inspect relevant
  diff/history in lazygit. Exclude others' dirty or claimed files from edits.
- Before probing, list the exact source paths, session/log references, literal
  commands, expected results, and stop conditions for this run. An unknown
  input is a question, not permission to broaden scope. No code lane is needed:
  the only authored outputs are the proposal and approval memos below.

2. Assess what works and what does not

- Trace each selected interaction from instruction/entry point through tool or
  procedure to its observed result. Keep the working parts: record at least
  one concrete success when evidence exists, including what must not regress.
- Separate **observed defect** (current expected/actual mismatch with a source
  or reproducible failure), **hypothesis** (plausible cause not established),
  and **tradeoff** (working behavior whose cost may merit a different choice).
  Missing evidence means unknown, not broken. Earlier conversation findings
  are leads until rechecked; never file them wholesale as verified defects.
- Use the smallest safe read-only observation or existing focused check.
  Record date, environment/revision, command or source location, actual result,
  and uncertainty. Do not run fixes, modify settings, install tools, restart
  services, spend money, or exercise destructive probes to prove a point.
  If proof requires such an action, propose the bounded investigation instead.
- Prefer no change or an existing mechanism over a new abstraction. For each
  candidate name the benefit, affected paths, regression risk, and a concrete
  success check. A hypothesis proposes investigation, not its guessed fix.

**Scan the ecosystem for what the harness does not have.** Everything above
finds what is present being wrong. A harness is also improved by what exists
elsewhere and is not installed — a Claude Code plugin, a CLI a routine
hand-rolls around, a crate that retires a script. Nothing in this
routine would ever surface one, so the scan is its own step and not an
afterthought of the inventory.

- Name the gap before searching, as [[scout]] requires: one sentence saying
  what is being picked and what it must beat. A scan with nothing to reject
  returns shopping, not evidence, and the gap comes from the inventory above —
  a step some routine performs by hand, a check nothing runs, a surface the
  record says is missing. Read
  `~/dev/infra/pearde/resources/scout/findings.md` first; a job answered there
  is read, not re-run.
- Rank on at least two of scout's four layers, never stars alone. Attention and
  Stars say a thing is talked about; Installs says people run it; Verdict says
  it is safe to depend on — `scorecard`, `osv`, `depsdev` and `eol` are the
  layer a proposal to *install* something needs and a popularity number cannot
  answer. Two axes that disagree are the finding: stars with no installs is a
  good README, and a live star count on an archived repo is what the `gh`
  route reports last push and state for.
- Every route is a read over the network and changes nothing here. Install
  nothing, add no dependency, edit no settings file: this routine's gate covers
  a tool exactly as it covers a patch, and an unapproved install is the one
  irreversible thing a scan could do.
- A surviving candidate becomes a proposal in step 3 like any other, carrying
  its numbers with the route that produced each, and a `Check` naming what
  installing it would have to demonstrate *on this tree* rather than in its
  own README. A candidate that loses is worth one line in the step 4 summary,
  so the next pass does not re-scan the same ground.

3. File only distinct proposals

- Compare the mechanism and intended outcome against existing work, questions,
  and decisions, including blocked, done, and rejected proposals. Reuse a
  matching approval thread instead of creating another. Never reopen rejected
  scope or alter another session's work. Existing executable work is reported
  as already tracked; do not silently block it or duplicate it.
- For each new focused proposal write `memos/work/<proposal>.md` using
  work-memo: `kind: work`, `level: 10`, `status: blocked`, `description`,
  `read_when`, and `## Do` / `## Check` body sections. Check leaf uniqueness
  with `rg --files memos -g '<proposal>.md'` before creating it.
- In `Do` state the evidence classification and source, exact proposed scope,
  benefit/tradeoff, working behavior to preserve, and a link to the approval
  question. State "Blocked pending explicit user approval", followed by the
  real approval memo's wikilink and "no implementation is authorized." Create
  that approval leaf before adding the link, then update its backlink after
  the work leaf exists. Do not set a
  claim or dispatch the item. Keep broad or unresolved designs blocked; do
  not pre-create executable children.
- In `Check` specify the literal command and expected result, or an exact
  reproducible observation, including the regression check for what works.
  Mark unrun checks as unrun. When a command cannot yet be specified, scope the
  item to finding that evidence; do not pretend the implementation is specced.
- Write `memos/intake/<approval-question>.md` as [[question]], one proposal per
  topic, with `----`, `Q:`, and `A: ?`. Link back to the work memo. Ask whether
  to approve that exact scope, revise it, or reject it; state the recommendation
  and its evidence/uncertainty. Use memo-writing's required frontmatter.
  Check leaf uniqueness before creating this file too.

4. Present and stop

- Author through `memo` writes and report the actual saved/index results and
  warnings. Indexes refresh automatically; never hand-edit them or fix unrelated defects.
- Commit what this run authored, so the proposal survives a sibling session:
  `git -C memos add` the exact proposal and approval memo paths written in
  step 3, then `git -C memos commit` immediately after, with a message naming
  this run. Never `git add -A`, never a path another session left dirty, and no
  generated index beyond what these memos' own regeneration touched
  (git-policy). Committing a proposal is preservation, not approval: the
  work stays `blocked`.
- Give a short "working / observed problems / unknowns" summary with sources,
  followed by concise numbered approval questions. Each question links to its
  approval memo and proposed work, names the evidence class, recommended scope,
  and verification. No quota: if nothing merits change, say so without filing
  work. Mention existing duplicates rather than asking for them again.
- All newly proposed work remains `blocked`. Stop for the user's answer: running
  this routine, silence, an agent's recommendation, a passed check, or approval
  of another item is never approval. Do not invoke [[work]] or [[drive]].

5. Record an explicit reply, without executing

- Resolve the numbered reply to the exact approval memo; ambiguous replies need
  clarification. Follow [[drill]]'s answer recording: preserve the user's exact
  words. Once settled, rewrite the question in the [[decision]] shape, dated,
  `status: decided`, preserving the answer and links. Approval or rejection is
  the decision's text, not a new work-status vocabulary.
- Only the explicitly approved, fully specified scope may become `status: open`;
  retain other blockers. Partial approval leaves the rest blocked. A changed
  proposal needs its own explicit approval before becoming executable.
- Rejection leaves work `status: blocked`, with the rejection decision linked
  and a line that only explicit user reconsideration unblocks it. Deferral or
  no answer leaves `A: ?` and work blocked. Never mark rejection as `done`.
- Record replies through `memo` writes; report warnings and the status
  moves. Execution is a separate user-authorized run of [[work]]; this routine
  never applies its proposed improvements.
