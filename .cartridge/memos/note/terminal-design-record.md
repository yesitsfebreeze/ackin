---
kind: note
description: The terminal interface design library is its own record under builtin/ui, not part of this one
date: "2026-09-11"
uses:
  - usage: "[[read-usage]]"
    when:
      - designing or reviewing any terminal interface in this repository
      - looking for the terminal design principles, element catalogue or previews
---

# Terminal design lives in its own record

`builtin/ui/.zirkle/memos/` holds the terminal interface design library: twenty
memos covering the medium, hierarchy, density, text measurement, colour,
structure, motion, states, regions, scrolling, focus, keys, input, terminal
capabilities, hygiene, accessibility, the element catalogue, prior art, the
shipping gate, and a verbatim copy of the Command Line Interface Guidelines. Its
entry is the `tui-design` note and its map is the `tui-design-library` scope.

It is deliberately generic: nothing in it is about this repository, so it can be
read, copied or published as a design library on its own. Run the drawn examples
with `just design` (`builtin/ui/previews/preview.py`); each element fills a real
terminal and `b` shows its behaviour spec.

Read it from that record, not from here: a session working in `builtin/ui` loads
it as the nearest record. This note exists so a session at the repository root
knows where it went.
