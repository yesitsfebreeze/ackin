---
kind: routine
description: Generate a report file (changelog, dep audit, file tree). Use when the user asks for a written
  report or exported summary.
uses:
- usage: '[[run-usage]]'
  when:
  - generate a changelog
  - audit dependencies
  - export a summary
  - file tree report
  tags:
  - report
  - docs
  - artifact
  - export
---

## Inputs

No prerequisites beyond the commands named below. Recipe parameters are named in Do; supply paths and arguments for the current task.

## Do

Each recipe writes its result to a file and prints `ARTIFACT <path>` as the last
stdout line; read the file at that path. For a quick stdout summary instead, use
[[agents-summarize]].

```just
# changelog from the last N commits
changelog count="50" out="dist/CHANGELOG.md":
  @mkdir -p dist
  @echo "# Changelog" > {{out}}
  @git log -n {{count}} --pretty=format:"- %s (%h)" >> {{out}}
  @echo "ARTIFACT {{out}}"

# deprecated symbols across the tree, as JSON
deprecations out="dist/deprecations.json":
  @mkdir -p dist
  @grep -rn "@deprecated" src | python scripts/deprecations_to_json.py {{out}}
  @echo "ARTIFACT {{out}}"

# markdown tree of a directory
tree root="src" out="dist/tree.md":
  @mkdir -p dist
  @find {{root}} -type f | sort | sed 's|^|- |' > {{out}}
  @echo "ARTIFACT {{out}}"
```

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `changelog` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
