---
kind: type
type: grammar
description: One overloaded name, its senses, and which one a reader should take
uses:
  - usage: "[[read-usage]]"
    when: [meeting an overloaded word in code, config or the record]
  - usage: "[[compose-usage]]"
    when: [writing a grammar memo, naming something near an overloaded word]
---

# Grammar memos

A `kind: grammar` memo disambiguates one word that does several jobs in cartridge:
the senses it carries, which authority owns each, and where the ambiguity
bites. One memo per word, named `<word>-grammar` (the `-grammar` suffix keeps
leaf names unique across the record).

Frontmatter: `description` is the one-line disambiguation. `overloads` lists
the senses, one phrase each. The body names each sense, points at its
authority (source file, config key or decision memo), and links the
neighbouring words the reader is likely confusing it with
(`[[other-grammar]]`). If two senses should not both exist, say so and link
the open question — a grammar memo may flag a name that needs streamlining,
but the rename itself is a decision, not a grammar edit.

Write a grammar memo when a word has caused a misread or an off-target
change, not for every noun. Naming something new? Read the grammar memos of
the words near it first; the record wins over habit.