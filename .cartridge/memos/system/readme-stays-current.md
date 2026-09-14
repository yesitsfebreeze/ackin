---
kind: system
description: Keep README.md in step with the program whenever its behavior or surface changes
---

`README.md` at the repository root is the public description of cartridge. Any
change to the program — CLI commands or flags, `just` recipes, requirements,
repository layout, profiles, manifest fields, settings, concepts or the set of
builtin cartridges — updates `README.md` in the same change. A change is not
finished while the README still describes the old behavior. When the matching
guide under `docs/` or `llms.txt` changes, check the README's summary of it too.
