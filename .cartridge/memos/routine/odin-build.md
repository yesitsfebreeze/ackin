---
kind: routine
description: Drive the Odin compiler — run, build, type-check, test, and document a package, plus the
  optimization/debug/vet/collection/build-mode flags. Use to compile an Odin package or single file, run
  it, run its @(test) procs, or produce a shared library. The syntax it compiles is in @odin/lang.
uses:
- usage: '[[run-usage]]'
  when:
  - build an odin program
  - run an odin file
  - compile odin package
  - run odin tests
  - odin check
  - odin compiler flags
  - odin optimization debug
  - build an odin dll
  - odin single file vs package
  tags:
  - odin
  - build
  - compiler
  - run
  - test
  - check
  - doc
  - toolchain
---

## Inputs

Requires on PATH: `odin`. Recipe parameters are named in Do; supply paths and arguments for the current task.
Risk: side effects file-write, process; danger medium.

## Do

The Odin compiler thinks in **directory packages**: `odin build <dir>` compiles
every `.odin` file in `<dir>` (one shared package, one `main`) into an
executable. To treat a lone file as its own package, append `-file` — sibling
files then become invisible. The same dir-vs-`-file` rule holds for `run`,
`check`, `test`, and `doc`. Program arguments go after `--`. The language itself
is in [[odin-lang]]; memory/error idioms in [[odin-runtime]].

| Recipe | Maps to | Does |
|--------|---------|------|
| `run` | `odin run` | build then execute (default) |
| `build` | `odin build` | compile to an executable |
| `check` | `odin check` | parse + type-check only, no codegen (fast) |
| `test` | `odin test` | build and run every `@(test)` proc |
| `doc` | `odin doc` | generate package documentation |
| `version` | `odin version` | print the compiler version |

Flags are colon-form `-flag:value`. The ones worth knowing, passed as trailing
`flags`:

| Flag | Effect |
|------|--------|
| `-file` | treat the path as a single-file package |
| `-out:name` | name the output binary |
| `-o:none\|minimal\|size\|speed\|aggressive` | optimization level (default `minimal`) |
| `-debug` | emit debug info (implies `-o:none`) |
| `-vet` | extra checks (unused, shadowing, …); `-strict-style` enforces the house style |
| `-define:NAME=value` | set a compile-time `#config(NAME, default)` |
| `-collection:name=path` | add an import collection (`import "name:sub"`) |
| `-build-mode:exe\|dll\|lib\|obj` | output kind (default `exe`) |
| `-target:os_arch` | cross-compile target (`-target:"?"` lists) |
| `-no-bounds-check` | drop bounds checks program-wide |

```just
# build a package dir (default ".") or single file (append -file) then run it; args after -- ; trailing flags pass through
run path="." +flags="":
  odin run {{quote(path)}} {{flags}}

# compile a package dir or single file to an executable
build path="." +flags="":
  odin build {{quote(path)}} {{flags}}

# parse and type-check only — no codegen (fast feedback)
check path=".":
  odin check {{quote(path)}}

# build and run every @(test) procedure in the package
test path=".":
  odin test {{quote(path)}}

# generate documentation from a package directory
doc path="." +flags="":
  odin doc {{quote(path)}} {{flags}}

# print the compiler version
version:
  odin version
```

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `run` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
