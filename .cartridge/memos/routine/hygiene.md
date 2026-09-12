---
kind: routine
description: One hygiene pass over the record — find the memo that has rotted, prove the rot with a command, and land the smallest edit that clears it.
uses:
  - usage: "[[run-usage]]"
    when: [the record is stale, duplicated or unreachable, or asking whether a memo still tells the truth]
---

1. Read

- `memos/SYSTEM.md`, then memo-layout, memo-writing and
  memo-atomicity — the three laws every probe below measures against.
- `git status --short` in both trees: this root and `memos/`, which is its own
  repository shared by every lane (`lanes-not-a-shared-tree`). A memo another
  session is mid-edit on is never this pass's candidate.
- Read existing structural warnings before choosing a candidate. Managed
  creates and updates go through `memo` writes; no live-record test is a prerequisite.

This routine reads the record and writes the record. [[improve]] adds what the
record does not yet hold; [[quality]] removes what the code does not need;
hygiene removes what the record holds and should not. A pass that finds rotten
code rather than a rotten memo writes the `work` memo and hands it to [[work]].

2. Probe

The caller may name one probe. With none named, walk in order and stop at the
first that yields a candidate — cheapest and most deletable first.

**a. Uncondensed intake and semantic bundles.** Read the candidate's claims;
headings alone do not establish that it needs splitting.

```
ls memos/intake/ | wc -l
grep -h '^kind: ' memos/intake/*.md | sort | uniq -c | sort -rn
```

`memos/intake/` is what has not been condensed yet (memo-layout); an intake older than the
daemon's daily `kern compact` is a fold that failed rather than a fold pending.

The fold is this pass's to land, not the daemon's to be waited for. The daemon
folds every kind at once through an 8k reason window, which is how one run
deleted 25 memos nobody had read
(compaction-folds-overflow-into-typed-folders-without-loss); a pass folds
one kind, a handful of memos, and reads the answer first:

```
kern compact --kind <kind> --limit 20 --dry-run
kern compact --kind <kind> --limit 20
```

A narrowed run leaves the day unstamped, so the daemon still owes the rest —
the pass is precision, not a substitute. `--dry-run` prints the claim titles
the fold would write, and a title that carries no number, path or symbol from
the memos it folds is a fold to refuse rather than land. Only a committed
intake memo is folded at all, so the tree's own `git status` is the check the
fold is reversible (`compact-deletes-what-git-has-never-seen`).

**b. A claim the code has falsified.** The record saying what the tree stopped
doing.

```
rg -l '^kind: (code-fact|knowledge)$' memos/
rg -o --no-filename '`[a-z_]+::[a-z_]+`|`[A-Z_]{3,}`' memos/code-fact memos/knowledge | sort -u
```

A `code-fact` is true until the code changes and is fixed in the same change
(code-fact) — so the probe is the symbol it names, grepped in `src/`. A
symbol the record names and the tree does not hold is a hit; the memo either
takes the new name or goes, and git is the archive either way.

**c. Documentation that no longer matches reality.** A row whose file is gone,
a recipe that no longer runs, a verb that was renamed.

```
just --list
kern --help
rg -o '\((system|routine|dashboards)/[a-z0-9-]+\.md\)' memos/*.md memos/system/*.md | sort -u
```

`kind: documentation` is the one kind whose trust rule is that it matches
reality (documentation), so a stale row here is a defect and not a
preference. The rot runs both ways — a help text naming what the record
renamed is the code lying about the record rather than the record lying about
the code, probe (b)'s verdict, landed by [[quality]].

**d. Two memos carrying one claim.** The duplication the atomicity law forbids
between files rather than inside one.

```
kern audit
kern doctor
rg -h '^description: ' memos/*/*.md | sort | uniq -d
```

`audit` ranks stored thoughts by noise likelihood and `doctor` names the
near-duplicate twins; two `description:` lines that read alike are the cheap
version of the same question. A hit is two memos a reader could cite
interchangeably. The merge keeps the better-named leaf, folds the other's
evidence into it, and the loser's links are repointed in the same change —
a link resolves by leaf name and nothing else (memo-index).

**e. A memo nothing reaches.** Written, indexed, and linked from nowhere.

```
rg -o --no-filename '\[\[[^\]|]+' memos/ -g '!_index_*' | sed 's/\[\[//' | sort -u > /tmp/linked
find memos -name '*.md' -not -name '_index_*' -not -path 'memos/dashboards/*' -exec basename {} .md \; | sort -u | comm -23 - /tmp/linked
```

`-g '!_index_*'` on the *link* side is what makes the probe able to answer at
all. The generated per-kind index wikilinks every memo it lists (memo-index),
so a sweep that reads those files finds every memo linked and returns an empty
set every time — measured 2026-09-08, the probe without it answered 0 of 856
and with it answered 159. `dashboards/` is pruned from the *file* side for the
neighbouring reason: nothing there is a claim and the gate does not read it
(memo-layout) — left in, its two generated views answer this probe every
pass and have to be re-adjudicated every pass.

`-o --no-filename`, never `-oh`: `-h` is rg's help, and the run that first wrote
this probe read a help screen as its answer and called 719 of 719 memos
unreachable. The character class needs its `\]` for the same reason — Rust's
regex refuses the POSIX `[^]|]` the shell would have accepted.

Managed authoring refuses newly broken body links; this is the other direction,
which nothing checks. An unreachable memo is not automatically rot — an index
entry finds it — but a claim no other claim leans on is either the connection
nobody drew or the memo nobody needed. Say which before acting.

**f. A declaration nobody calls.** The surface the record declares and no agent
uses.

```
python3 - <<'EOF'
import json, glob, os, re
c = json.load(open('.kern/data/reflex.json'))['counts']
for p in glob.glob('memos/**/*.md', recursive=True):
    m = re.match(r'---\n(.*?)\n---', open(p).read(), re.S)
    if not m:
        continue
    kind = dict(l.split(': ', 1) for l in m.group(1).split('\n') if ': ' in l).get('kind')
    if kind in ('routine', 'persona'):
        name = f"{kind}:{os.path.basename(p)[:-3]}"
        print(c.get(name, 0), name)
EOF
```

Every declared routine and persona is a tool counted in the reflex ledger under
its own name (a-persona-is-worn-through-its-tool). A declaration at zero
across a long ledger is either a handle nobody found — a naming defect, fixed in
the `description` — or a procedure nobody wants, which git can hold instead.
A young declaration is not a hit; say how old it is before calling it one.

**Read the ledger by absence, not by zero.** `Reflex::record` is
`counts.entry(name).or_insert(0) += 1` (`src/rpc/src/reflex.rs:85`), so a name
enters `counts` on its first use and a never-called tool has no row at all —
printing `counts` and scanning it for a zero returns the one answer the probe
cannot mean. The script above supplies the declared names and defaults each
missing one to 0, which is the same shape `Reflex::draw` uses (`:104`).
And zero is not automatically dead: the count rises only when the name reaches
`Server::invoke`, so a routine a session runs from its skill handle rather than
through `kern mcp` scores zero however often it runs
(every-routine-and-persona-scores-zero-in-this-repos-ledger).

3. Pick

One candidate per pass. Say in one line which probe found it, what has rotted,
and what clearing it costs a reader. Prefer, in order: a falsified claim (the
record lies), a stale document (the record misleads), a duplicate claim (the
record splits a reader's trust), a bundle (the record cannot be cited), an
unreachable memo, an uncalled declaration.

4. Prove

Rot asserted is not rot found. Before editing, produce the command whose output
is the finding — the grep that returns nothing, the recipe that errors, the two
descriptions side by side, the ledger count — and quote it. A memo removed on a
reading rather than a run is a memo the next pass writes back.

5. Land

`memos/` is one record shared by the trunk and every lane, so a pass that edits
only memos needs no lane; a pass whose fix reaches code opens one
(`just lane hygiene-<probe>`, `just land`).

- The smallest edit that clears the rot. A split for a semantic bundle, with
  both halves linking each other. Create the new leaf before adding links to
  it, then update its sibling. A merge for
  a duplicate, with every inbound link repointed. A correction for a falsified
  claim — an edit adds, and a superseded decision keeps its text while the new
  one names what it supersedes (memo-writing).
- A deletion where the claim is gone rather than wrong. Git is the archive.
- Create and update through `memo(op="write", path=…, body=…)`; inspect its
  warnings and saved/index status. Successful managed writes refresh indexes;
  no index is authored by hand (memo-index). Direct deletions, renames,
  editor saves, and compaction bypass that refresh; follow them with a managed
  write of an affected surviving memo and verify the resulting indexes.

6. Record and loop

- The pass itself is not a memo. What the pass decided may be: a duplicate
  deliberately kept, a memo judged unreachable on purpose, a claim re-dated
  rather than deleted — one `decision` memo so the next pass does not re-argue
  it.
- One line: probe, what had rotted, what landed.
- Back to step 2 with the next probe. The loop ends when a full walk of the six
  yields nothing, or when the caller stops it.
