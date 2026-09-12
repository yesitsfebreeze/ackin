---
grammar: cartridge
subject: the words this repo gives a meaning of its own
date: 2026-09-12
updated: 2026-09-12
---

# Grammar — cartridge

Every word this repo gives a meaning of its own. One vocabulary, so a session,
a worker and a person name the same thing the same way.

Reader: an agent, cold. A row is the meaning, never the rule; the rule stays
in the file the group heading names.

**A word earns a row where its meaning here differs from ordinary English, or
where it stands beside another a reader confuses it with.** A word carrying its
everyday meaning gets no row.

The groups down to **Words that collide** ship with the board — pearde's own
vocabulary, the same everywhere. **This repo** at the end is yours, and the
half that makes this file worth reading. @references/grammar.md is the format
and the rules for growing it.

## The board

@references/parts/board.md · @references/parts/contract.md

| term | is |
|---|---|
| **board** | `.pearde/` in a repo — `settings.md`, `vision.md`, `prds/`, `memos/`, `workflows/`. What a session works |
| **PRD** | a directory under `.pearde/prds/` holding `prd.md`. The unit of work, and the only thing carrying a `state` |
| **child** | a PRD inside another PRD's directory. Written by `refine`, never by hand |
| **leaf** | a PRD whose children are all `done`. The only shape a worker is dispatched on — work flows to the leaves |
| **container** | a parent whose children are all `done` and which holds no spec and no open box of its own. `collect` closes it; `claim` refuses it |
| **spec** | `specs/specNN.md` — one implementable unit, written by the analyst from a build it ran |
| **acceptance box** | a `- [ ]` line in a spec. `[x]` only for a check that ran, quoting output — and ticked as it closes. The board's only live view of a worker |
| **verify block** | a spec's `## Verify and Proof`. Re-run by `collect`, never taken on the report's word |
| **probe** | the analyst's own build attempt, left uncommitted at `.pearde/prds/<prd>/probe/`. Pass one — the implementer continues it |
| **contract** | what exists when the PRD is done, in the PRD's body. Also the **frontmatter contract** — the keys the tools read, and what a missing one defaults to |
| **frontmatter** | the `---` fenced key block. Unknown keys are the user's: preserved, read by nothing |
| **footprint** | the paths a PRD writes. The overlap check before dispatch, and the commit's scope |
| **repo** | where the code lands. A board may plan a tree it does not sit in |
| **vision** | `.pearde/vision.md` — one sentence, `terminals:`, `edges:` |
| **terminal** | a PRD whose completion is the vision |
| **axis** | the vision axis: a PRD is on it when a terminal is reachable from it. `axis: asap` is the lane that dispatches before everything |
| **depth** | the longest serial chain from a PRD to a terminal. Deepest dispatches first |
| **master board** | a board whose `settings.md` carries `members:`. It plans across other boards and moves no file in them |
| **member** | a board a master merges. Keeps its own PRDs, settings, memos and view |
| **archive** | `.pearde/prds/archive/<name>.md` — a done PRD flattened to one file. It holds no `prd.md`, so `scan` walks past it |

## States, and what moves them

@references/parts/states.md

| term | is |
|---|---|
| **state** | one of the nine below, in `prd.md` frontmatter. Written by a command, never by hand |
| `open` | claimable for analysis |
| `analyzing` | an analyst is working out what to do |
| `refine` | the PRD holds more than one contract, or one too large. It needs children |
| `question` | the board is stopped on a person, and the PRD says what it is asking |
| `specced` | specs are on disk and the weight is summed. Ready for an implementer |
| `claimed` | an implementer holds it |
| `blocked` | the work is done and a box waits on something named. Live work, counted, carrying `needs:` |
| `done` | every box closed, every verify green, the commit landed |
| `failed` | the attempt did not produce the work |
| **parked** | any `state` outside the nine. Never dispatched, never scheduled, in no count, reported by name. `release <prd> open` is the way back |
| `deferred` | the one parked state the board writes itself — a derived PRD that cannot name what it gets wrong |
| **transition** | one state change. A command checks its gate, writes the state, prints the progress line, and exits 1 naming the gate when the table forbids the move |
| **gate** | the precondition a command checks before it writes. Also `gate:` in `settings.md` — one command `collect` runs in the repo root before the commit |
| **forced** | `set --force`, a transition past its gate. The line says `forced`, and the view's drag says `forced · view` |
| **claim** | `claim:` on a PRD — which worker holds it, since when. `pearde claim` is what writes it |
| **silent** | a claim whose files have not moved for longer than `claim-ttl`. Read off mtimes, never off a process — the board cannot see a worker |
| **sweep** | the pass over silent claims: `analyzing → open`, `claimed → failed`. Never a claim the pass file names |
| **rider** | board state written between transitions — an answer, a split, a memo. It rides the next commit, and the line says `rides <path>` |
| **widen** | `collect --widen <path>` — taking a dirty path the claim predates, named on a `widen:` line in the message |

## Weight and order

@references/parts/order.md

| term | is |
|---|---|
| **weight** | what the progress line and the plan size a PRD by. First that answers: its specs' `complexity`, its own, its `est`, the board average, `weight-default` |
| `complexity` | 1-100, scored by the analyst at spec time from the specs it just wrote |
| `blast-radius` | `high` · `mid` · `low` — what breaks if the PRD is wrong. Breaks ties, and decides what a pass leads with |
| `est` | a fallback weight for a PRD with no `complexity`. Nothing asks an analyst to produce one |
| `actual` | what one PRD took. A record: the plan never schedules by it |
| **calibrate** | the fit of one machine-wide constant — hours per unit of weight — from every `done` PRD carrying `actual:` across every registered board |
| `TUNE` | the hand-set margin in `mapfile.py`, 1.618. Tuned hours are weight × the fit × this |
| **dispatchable** | `plan.dispatchable` — the one predicate `scan`'s ready band and `claim` both read, so what the scan offers is what `claim` takes |
| **pressure order** | the one ranking of the whole board: to collect, red, refine, waiting on you, in flight, ready now, gated, parked, landed. The cut is between 1 and 2 — above it is this pass's, below it is already somebody's |
| **to collect** | every box closed, a worker still holding it. No dispatch is cheaper |
| **red** | `failed` — a lane with commits and a `## Failure`; `retry` then `claim` puts a worker back on it, before anything open is dispatched |
| **waiting on you** | `question` — the one state that moves only when a person moves it; `blocked` waits on other tickets under gated, `refine` is the pass's own step |
| **in flight** | a worker holds it and its boxes are ticking |
| **ready** | dispatchable this second, one lane per file — a footprint overlapping a lane in flight, unmerged, or offered above it waits in gated as `after <prd> (footprint)`. Inside the band, biggest door first — that ordering *is* the dispatch order |
| **gated** | the rest of the plan, in schedule order |
| **landed** | `done`, laid out to the left of now |
| **critical path** | the chain that sets the finish. Weight cut there moves the vision closer; weight cut anywhere else moves nothing |
| **float** | how late a PRD may start before it becomes critical |
| **cpm** | the critical-path arithmetic in @resources/board/render.py, and the key it puts in the payload |
| `needs` | a hard gate — PRD directory names that must be `done` first |
| `edges` | a dependency in `vision.md` that nobody wrote as `needs:` |
| `priority` | the user's importance. Breaks ties within one depth |
| `origin` | `requested` — the user asked; `derived` — the board found it while working |
| `from` | which PRD's work surfaced a derived one |
| **tripwire** | live derived PRDs reaching the count of requested ones. The board is working on itself: stop filing, report both counts, put it to the user |

## Who works

@references/parts/roles.md · @references/parts/workers.md · @references/parts/dispatch.md

| term | is |
|---|---|
| **orchestrator** | works the board, and is the only writer of PRD state. One per board |
| **worker** | a dispatched agent that does one job and hands back one line |
| **analyst** | turns one `open` PRD into specs, a split, or questions — by building it, never by reading |
| **implementer** | turns a `specced` PRD's specs into verified code |
| **consultant** | a persona called mid-pass. Reads the session and the board, answers, writes nothing |
| **pass worker** | `pearde-pass` — the window the loop runs in. Ends by `transitions-per-pass` and hands back one verdict |
| **dispatcher** | the session the user asked. Dispatches pass workers, carries the user's answers between them, holds no board state |
| **pass** | one stretch of the loop in one window. `once` is one pass |
| **pass file** | `.pearde/.state/pass.md` — the session's own memory, rewritten whole at every transition. Machine-local, git-ignored |
| **brief** | the exact text a worker is handed, printed by `pearde brief`. Nothing about it is composed by hand |
| **placeholder** | `<name>` in a brief block. `brief` fills the set @references/parts/workers.md names, and nothing else |
| **report** | `.pearde/prds/<prd>/report.md`, written by the worker. What comes back is one line |
| **verdict** | the one word on a `Verdict:` line in a report's first 40 lines. It picks the transition, and `collect` is what reads it |
| **persona** | who is working — what is noticed first, what is pushed back on, what counts as done. Session state in `PEARDE_AS`, held on no board file |
| **role** | what the session does. The role is the job; the persona is who holds it |
| **id** | the word typed for a persona — `engineer`, `designer`, `mentor`, `skeptic` — and what the status line shows |
| **split** | the analyst's `## Split` table. `pearde refine` reads it and writes the children |

## The closed sets

A verdict outside its set is refused with nothing written.

| term | is |
|---|---|
| **analyst verdict** | `SPECCED` · `REFINE` · `QUESTION` |
| **implementer verdict** | `DONE` · `BLOCKED` · `FAILED` |
| **pass verdict** | `MORE` · `ASK` · `DRAINED` · `BLOCKED` |
| **memo kind** | `decision` · `note` · `invariant` |
| **memo status** | `open` · `decided` · `superseded` |
| **on failure** | `stop` · `→ N`, N below the step's own number |
| **outcome** | `passed` · `failed → N` · `stopped` |
| **doctor verdict** | `ok` · `off` · `broken` |

## Asking

@references/drill.md

| term | is |
|---|---|
| **drill** | interviewing until the request is a contract, recorded as a PRD tree |
| **frontier** | every decision whose prerequisites are settled — what is askable now without guessing at answers not yet heard |
| **fork** | one question: two sentences and a question mark — what is being chosen, and what it changes for the person answering |
| **prepared answer** | one of exactly three complete answers under a fork, each one plain sentence of what they get. The best is first, marked `(recommended)` |
| **technical anchor** | the HTML comment under the third answer — which files, which slug, which spec the answer lands in. Nothing that shows a question to a person shows it |
| `## Questions` | a pass of forks in the PRD, in the format above. Written only with content under it |
| `## Answers` | what came back, `**Q1** — <the decision>`, numbers matching |
| `## Asked` | the pass file's live frontier: what went out, and whether it came back. A question here is never re-put |
| **ask.md** | `.pearde/.state/ask.md` — the forks a pass worker leaves for the dispatcher when it hands back `ASK`. The one file the dispatcher opens |

## Records beside the PRDs

@references/memo.md · @references/workflow.md

| term | is |
|---|---|
| **memo** | `.pearde/memos/<slug>.md` — what was decided and what it beat. No `state`, never dispatched, on the board anyway |
| `kind: invariant` | the testable memo: a rule that must keep holding, carrying the `verify:` command that exits 0 while it does. Filed proven, never on faith |
| **superseded** | a memo replaced by another, naming it in `superseded_by`. Superseded invariants no longer bind |
| **kind index** | `memos/README.md` — generated from the tree, never maintained. The check fails when it is stale |
| **workflow** | `.pearde/workflows/<slug>.md` carrying `workflow:` — `## Use when`, then `## Steps`, an ordered list of atomics |
| **atomic** | a file in the same directory carrying `atomic:` — one unit: `## Do`, `## Done when`, `## Fails when`. One that needs "and then" is two |
| **library** | the workflow directory, `workflows:` in `settings.md`. It never merges: on a master, a slug resolves against its own board first, then the master's |
| **route** | a workflow followed by one worker. `## Route` in an analyst's report is a route drafted where nothing in the library fit |
| **back-edge** | a step's `on failure` returning to an earlier one. Taken at most twice per run; the third failure is `stop` |
| `runs` | how many runs a file was in — one collect, one count, never traversals. Evidence that a file is exercised, not a score |
| **edit** | a worker's replacement text for a section an atomic got wrong. Applied when the failure was the atomic's, refused when it was the code's or the PRD's — and the refusal is written down |
| **fold** | replacing the sentence that was wrong, rather than logging beneath it. Git holds what it replaced |
| **report** (the document) | `.pearde/report.md` — the board written for a person, rewritten whole. One state, never a log, and the only document on the board a person is the reader of |

## Knowledge and graphs

@references/knowledge.md · @references/graph.md · @references/obsidian.md

| term | is |
|---|---|
| **knowledge base** | `.pearde/wiki/` — what the board learned from outside the repo, as linked notes with provenance. Machine-local |
| **source** | one external finding, raw, arguing nothing of ours, carrying the route id or URL that produced it |
| **conclusion** | a synthesized answer naming every source it derived from. Fewer than two sources is a hunch, and the tool refuses it |
| **pending** | a research question queued by a `query` that found a gap. Deleted when it stops mattering, never drained to zero |
| **wikilink** | `[[slug]]` — what holds the note graph together. `relink` resolves them and symmetrizes `related:` |
| **vault** | the board seen through Obsidian, rooted at `.pearde/`. A vault opens by URI only once it is in Obsidian's own register |
| **graphify** | the tool behind `@@graph`: tree-sitter AST for code, a semantic pass for docs, into `graph.json` |
| **god node** | a node the graph connects to far more than the rest — what `GRAPH_REPORT.md` names first |
| **corpus map** | graphify's graph, over the repo. Not the note graph, which is hand-built over the knowledge base |

## Scout

@resources/scout/README.md

| term | is |
|---|---|
| **bucket** | one line of `buckets.txt` — a name and the query that measures it. The knob |
| **snapshot** | one day's star counts, a TSV per sweep. The delta is diffed from our own, because the stargazers API is gone |
| **route id** | one ranking page, addressed by id and run by `route.sh` out of `routes.md` |
| **findings** | the dated record of what won, on which axis, and when |
| **reading list** | the curated list, each entry mapped to the mechanism it teaches |

## Addressing

@index.md

| term | is |
|---|---|
| `@<path>` | one file — the real path from the repo root with `@` in front. Nothing to look up |
| `@@<keyword>` | one scope — everything a feature is made of. A row in @index.md, never a directory |
| **scope** | what a feature is made of, not a reading list. The first anchor in the row explains the rest |
| **manifest** | @references/files.md — every tracked file, one row. Read when a file is added or moved |
| `@<member>/<prd>` | a member's PRD, board-wide. A PRD directory is never named `@…`, so the address cannot collide |
| **slug** | a lowercase hyphenated name that is both a filename and the key inside it. `doctor` fails when the two disagree |
| **handle** | a word the board answers to — @references/parts/handles.md. Several are also skills of their own |
| **skill** | one file under `references/skills/`: frontmatter deciding when it fires, a body pointing into `references/`. The knowledge is never in the skill |
| **install** | the set of links from a skills directory into this tree. Updating is re-linking, never a copy |

## The view

@references/parts/view.md

| term | is |
|---|---|
| **the view** | the local service at `127.0.0.1:8443` that renders every registered board live, and writes edits back to the files |
| **now strip** | the three doors above every view — `to collect` · `waiting on you` · `in flight`. Zero renders dimmed, never absent |
| **door** | any number that names a set of PRDs. Clicking it goes there, and the URL follows |
| **pill** | one PRD's bar. Its name rides inside it, or floats off the end when it cannot fit |
| **ghosted** | the part of a bar whose boxes are not closed yet. The edge moves as checks land |
| **row rail** | the slider on the plot's left edge, between every row at reading height and the whole board on one screen |
| `ppu` | pixels per unit of the x axis — weight in `vision` mode, days in `dates` |
| **seam** | where a board's own element renders: `toolbar`, `sidebar`, `inspector` |
| **deep link** | `#prd=`, `#view=`, `#state=`, `#crit=1`, `#collect=1` — where you are, as a link you can send |
| **gantt** | the same render as one self-contained HTML file, with no service and no edits |

## The machinery

@references/parts/guard.md · @references/parts/doctor.md · @references/parts/progress.md

| term | is |
|---|---|
| **progress line** | the one line printed on every state change, `▸ <prd>: <from> → <to> · …`, computed by the tool from the board after the write |
| `as <id>` | the persona, last on the progress line and never omitted — the only record a session's persona has |
| **status line** | the same numbers rendered continuously in the terminal, for a person watching. Nothing the loop reads |
| **guard** | @resources/guard.py — the loop's rules as a hook that refuses what is provably redundant, and counts what a transition cost |
| **stamp** | what the guard compares against: the newest mtime under the board. An unchanged stamp means an identical answer |
| **floor** | the smallest window a session was billed for. `context-budget` is measured over it, never from zero |
| **doctor** | one line per part, each `ok`, `off` or `broken`, a broken one carrying the command that repairs it |
| **part** | one row of that report — `skills`, `index`, `board`, `memos`, `workflows`, `grammar`, `health`, `questions`, `view`, `guard`, `vault`, `harnesses` |
| **harness** | a board's own `verify.sh`. Opt-in in `doctor`, because it is the one row measured in tens of seconds |
| **pinned** | a harness asserting its own executed total against a literal. An unpinned one prints a smaller total and exits 0, which is indistinguishable from success |
| **health** | how much a file resists being worked on — 1-100, 100 healthy, from its lines, branching, longest function and the graph's fan-in, fan-out and links. @references/health.md states the pointer-not-verdict rule; not restated here |
| **unhealthy** | a file scoring under `health-floor`. The one the implementer's brief names |

Every board knob is @references/settings.md. A key is a word here only where
this file gives it one.

## Words that collide

The pairs a cold reader gets wrong — one spelling, two meanings, per row.

| the word | here | and here |
|---|---|---|
| **state** | one of the nine on a PRD | `status:` on a memo is a different closed set, and `pearde status` is the read-only handle |
| **pass** | one stretch of the loop in one window | a drill asks its whole frontier in one pass, and a workflow step's `outcome` is `passed` |
| **report** | `.pearde/prds/<prd>/report.md`, a worker's, for the orchestrator | `.pearde/report.md`, the board's, for a person. `## Report` is the worker's, posted onto the PRD |
| **blocked** | the `blocked` state — work done, a box waiting on something named | `BLOCKED` from an implementer that hit a wall, and `BLOCKED` from a pass worker that cannot move the board |
| **collect** | the command that verifies, commits and closes | the pressure band of PRDs waiting for it |
| **sweep** | the pass over silent claims | a scout run that snapshots every bucket |
| **route** | a workflow one worker follows | a scout **route id**, one ranking page |
| **gate** | the precondition a command checks | `gate:` in `settings.md`, one command `collect` runs before the commit |
| **verify** | a spec's `## Verify and Proof` | `memos.py verify`, running an invariant's command; and `verify.sh`, a board's harness |
| **done** | the terminal state | `## Done when`, an atomic's checks |
| **index** | @index.md, the map of this repo | `memos/README.md`, the generated kind index |
| **vault** | the board through Obsidian, rooted at `.pearde/` | graphify's own vault under `.pearde/graphify/obsidian/`, deliberately outside it |
| **weight** | the number the plan sizes a PRD by | never hours. `est` and `actual` stay out of the schedule, which is what makes them honest calibration data |
| **claim** | `claim:`, which worker holds a PRD | a contested assertion in a report, which owes `reproduced`, `refuted` or `unmeasured` |
| **container** | a parent `collect` closes | never a runtime container |
| **parked** | a state outside the nine, scheduled by nothing | not `blocked`, which is live work, and not `failed`, which is an attempt that did not produce the work |
| **floor** | the smallest window a session was billed for, which `context-budget` is measured over | `health-floor`, the score under which a file is unhealthy |
| **complexity** | the PRD weight, 1-100, hand-written by the analyst — what the board schedules by | never a file's health, which is measured by a tool and schedules nothing; `branching` is the axis that would otherwise wear the word |

## Words a person never sees

Everything here is the orchestrator's vocabulary. None of it appears in a fork,
an answer label, or answer text — @references/drill.md is the rule and
@resources/questions.py refuses a pass that breaks it. Nor does a backtick, a
path, a file extension, a PRD slug, or a `Q<n>` cross-reference.

`.pearde/report.md` carries the same ban: no PRD name, no state, no weight.

## This repo

The words this project gives a meaning of its own. Empty is honest on a board
whose domain has no vocabulary yet; one row beats discovering the word twice in
conversation.

| term | is |
|---|---|
| cartridge | One program that does one thing and declares two facts: what it provides and what it needs. Written in any language, tested alone, replaceable on its own. |
| ledger | The registry of every cartridge installed on this machine. What the resolver reads. A cartridge that is present is askable; there is no list to edit. |
| resolver | The walk from a named tool down its dependency chain to the far end, and the launch of the first program that chain requires. |
| re-entry | The binary invoking itself for the next tool along a chain. One re-entry per link; the chain assembles from the bottom. |
| chain | A tool's dependencies in launch order, far end first. Resolved at runtime from the ledger, not declared anywhere as a whole. |
| tree | The running processes. Identical to the dependency tree because a dependency launches its dependent, and so it cannot drift from it. |
| wire | The JSON-lines connection a node inherits from the dependency that launched it. No discovery, no broker, no address. |
| host | The binary. Installed once, holds no features, does nothing on its own. Not a supervisor — nothing sits above the tree owning it. |
| stream | The append-only JSON event log a cartridge publishes to. One object per line, ordered, replayable — replaying it twice reaches the same state twice. Never the ledger, which is the registry of installed cartridges. |
| channel | A named topic on the stream. A cartridge publishes to it without knowing who listens; another subscribes by name and receives everything on it in order. Subscribing and unsubscribing are themselves events on it. |
