---
kind: routine
description: Write a new routine — name the job, gather every ingredient it needs, prepare them as one linked block, register the handle, prove the gate.
uses:
  - usage: "[[run-usage]]"
    when: [adding a procedure, or turning something done twice into a routine]
---

1. Read

- memo-writing, memo-atomicity, and memo-index,
  routine-is-a-procedure and routine-is-prepared-context.
- The routine nearest the job — [[work]], [[drive]], [[improve]] — as the
  shape to copy.

2. Name the job

One sentence: what a run of this produces, and when a caller reaches for it.
If the sentence needs an "and", it is two routines. Pick the kebab-case
handle; it is the file name, the `name:` and the slash command.

3. Gather the ingredients

This is the work. Walk every shelf and write down what this job actually
touches — nothing speculative, nothing left implicit:

- **Memos** — the generated index described by memo-index is the catalogue. Every memo the run
  must read, by leaf name, linked.
- **Gates and recipes** — `just --list`. Name the exact recipe, never "run
  the tests".
- **Kern operations** — `kern <verb> --help`, or the `mcp__kern__*` tools in
  an MCP session. Name the verb and the flags the job uses.
- **Commands** — the literal `rg`, `git` or `gh` invocation, copy-pasteable,
  not a description of one.
- **Files** — anything under `scripts/`, `.claude/` or the tree the run
  opens, by path.
- **A lane** — a routine that edits code opens one before its first edit
  and lands it at the close (`just lane`, `just land`,
  lanes-not-a-shared-tree); a routine that only writes memos needs none.

4. Write the memo

`.cartridge/memos/routine/<handle>.md`, frontmatter `kind: routine` and `name:`,
numbered steps in the body. Each step names its ingredients inline — the read
step lists paths, the check step lists the recipe, the write step lists the
kind and the destination folder. No ingredient appears for the first time
mid-run.

5. Register the handle

One `.agents/skills/<handle>/SKILL.md`, and nothing else —
`.claude/skills` is a symlink to that tree, so the file is written once
(one-skill-file-per-routine-under-two-harness-names). Its body calls
`mcp__kern__routine_<leaf>` rather than naming a path, because
`tool_declaration` locates the memo by leaf and the call is what the reflex
ledger counts. No `.claude/commands/` file is written; that directory was
deleted. The skill is a doorbell; the memo is the procedure.

6. Check

- Save through `memo(op="write", path=…, body=…)` and verify its generated index entry.
- The search test: read the routine as a cold agent and mark every point it
  would have to go looking. Each mark is a missing ingredient — go back to
  step 3.
- Report the managed-write result and any warnings. Report one line: handle, what it produces.
