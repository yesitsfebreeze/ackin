---
kind: type
type: persona
description: One worn preprompt — who is working, in the second person, with the practitioners each behaviour is taken from
uses:
  - usage: "[[read-usage]]"
    when: [wearing a persona, asking one a question, or dispatching work to one]
  - usage: "[[compose-usage]]"
    when: [writing a persona memo, or merging research into an existing one]
---

# Persona

A memo with `kind: persona` is a preprompt: the whole body is text an agent
reads and then *is*, for the rest of the run. Not a description of a role and
not an output style — a persona says what gets noticed first, what gets pushed
back on, and what counts as done. The leaf is the handle, one lowercase word
naming the profession rather than the person, because the handle is what a
caller types and `name` is who that handle is.

This declaration is the workspace's own. The memory cartridge ships a persona
declaration too, and the workspace one is authoritative where both exist — the
record must validate whether or not that cartridge is enabled, which is the
concrete reason this file exists rather than being inherited.

## Fields

`kind: persona`, a nonempty `description`, `name` (the person's name),
`profession`, and `read_when` naming the situation that should surface it.

## Body

Written in the second person — "you read before writing" — because a persona is
worn and never described. No pronoun stands for the person named in
frontmatter, so none is assumed.

- `## How you work` — bold-led bullets that are behaviours, never adjectives.
  Every bullet closes with `[<Name>: <trait>]` repeating a `Built from` trait
  character for character, so a reader tells a measured practice from a
  preference.
- `## Voice` — what this person never says.
- `## Built from` — one bullet per researched practitioner: who they are, what
  they are known for, the one trait taken, and the artefact documenting it.

A behaviour tracing to nobody is cut. A practitioner backing no behaviour
leaves.

## Three ways to reach one

| move | is |
|---|---|
| **wear** | read the body and work as that person for the rest of the task; the session's own judgement is replaced, not consulted |
| **ask** | put one problem to one persona and answer in that voice, keeping the session's judgement — say which persona answered, and do not let the answer become the default |
| **delegate** | dispatch a subagent whose whole prompt is the body plus the job; what comes back is that person's work, and the session reviews it as such |

## Writing one

A persona is composed from research and never invented: read the field's
working practice, find the named practitioners, take one nameable trait each,
and compose one fictional colleague under a name of its own whose first body
line says it is a composite. No reader may take a real person to have said
this, and none is quoted.

A leaf duplicating an existing one is a merge, not a new file: fold the new
research into that file's `## Built from` and record what changed in the body.

## Trust

A persona is worn or not at all — the same rule [[routine]] carries, for the
same reason. Its sections organize the preprompt; heading names are not an
atomicity test. A claim a persona makes in its own voice is not a memo until
someone writes it as one.

No tool in this profile serves personas: there is no `persona:<leaf>` key, so
wearing one means reading it. Naming the handle is enough for a caller who
knows the record; a profile that wants the handle must provide the key.
