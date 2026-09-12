---
kind: note
description: "Source paths, regression cases, scope and license obligations for selective DeepSeek plugin ports"
uses:
  - usage: "[[read-usage]]"
    when: ["Implementing fs, shell, agent, policy or context from the pinned reference"]
    tags: [documentation]
---

# deepseek-plugin-port-map

Inspected on 2026-09-09. Source: [deepseek-ai/deepseek-harness](https://github.com/deepseek-ai/deepseek-harness), pinned to [`b2e3b2a0125854567a4a5fcba75782e42fe84901`](https://github.com/deepseek-ai/deepseek-harness/commit/b2e3b2a0125854567a4a5fcba75782e42fe84901). References below are relative to that immutable commit, not moving master.

## Port boundary

The upstream is TypeScript with Cordis plugins. Port selected behavior and regression cases into glue's independently registered plugins; do not copy the framework or import its application as one plugin. In particular, `plugins/agent/` is the replaceable loop process, `plugins/harness/` builds context, router owns providers, sessions owns persistence, Lua policy decides permissions, and fs/shell own actual I/O.

Upstream separates `packages/core/agent` (the public agent contract) from `packages/core/agent-loop` (the driver). Glue can keep that distinction as its JSON service contract plus one process implementation without introducing an additional registry crate.

## Selected sources and checks

| Local work item | Upstream implementation | Regression cases to translate |
| --- | --- | --- |
| fs-tool-plugin | `packages/fs/tool-fs/src/{read,read-render,write,edit,error}.ts`; `packages/fs/fs-local/src/{index,fsio}.ts` | `packages/fs/tool-fs/tests/integration.spec.ts`: stale observations, unread overwrite, ambiguous edit, absent-file recreation. `packages/fs/fs-local/tests/filesystem.spec.ts`: concurrent creator, symlink identity, same-size rewrite with restored mtime, permission preservation. |
| fs-tool-plugin search | `packages/fs/tool-fs-search/src/{glob,grep,search-core}.ts` | `packages/fs/tool-fs-search/tests/tools.spec.ts`: direct argv, `--no-config`, leading-dash arguments, exit 1/no matches, explicit truncation and malformed backend output. |
| shell-tool-plugin | `packages/shell/tool-bash/src/{index,render}.ts`; `packages/shell/bash-local/src/index.ts` | `packages/shell/tool-bash/tests/tools.spec.ts`: timeout remains timeout even when a TERM trap exits 0, pre-abort prevents spawn, workdir resolution and nonzero status as a tool result. |
| agent-run-control / agent-loop-plugin | `packages/core/agent-loop/src/{index,agent,tool-calls}.ts`; `packages/core/agent/README.md` | `packages/core/agent-loop/tests/tool-calls.spec.ts`: stable call/result identity, ordered results, stop dispatching on cancel, drain started operations. Keep initial local scheduling serial; do not port parallel barriers yet. |
| harness-plugin | `packages/context/agent-instructions/src/{index,files,render,state}.ts` | `packages/context/agent-instructions/tests/agent-instructions.spec.ts`: deterministic scope and deduplication, multibyte budget edges, delimiter escaping, no duplicate injection on resume. Nested-file instruction refresh is deferred; current scope is the session cwd ancestry. |
| policy-plugin and fs mutation guards | `packages/fs/fs-observation-policy/src/index.ts`; `packages/guard/timeout-policy/src/index.ts` | Unseen/absent/present-version distinctions; required policy absence fails closed. Lua decision/deadline intent does not replace Rust mutation validation, process termination or cleanup. |

## Semantics to preserve and limits to state

- Owner-scoped file observations reset on plugin reload/new run. A partial read establishes a freshness version, not proof that the whole file was viewed. Unseen write is create-if-absent; unseen edit fails; a stale version requires rereading. Filesystem enforcement belongs in fs, with permissions still decided by Lua policy.
- Atomic no-clobber creation protects a concurrent creator. Per-target locking and guarded replacement are process-local, not a cross-process compare-and-swap guarantee. A local malicious process can race filesystem checks; neither upstream cwd nor this plan's path validation is a sandbox.
- Cancellation is not rollback. Drain started operations, record actual outcomes, mark unstarted calls cancelled-before-dispatch, and never automatically repeat uncertain side effects.
- Upstream sandbox escalation documentation promises more than `packages/sandbox/sandbox/src/escalation.ts` enforces: the helper does not receive an exact command/path, turn or prior-denial token. Do not port those promises as facts. Local approval is explicitly bound to immutable session/run/call/input and consumed once.
- Repository context framing is not a security boundary. Escape framing delimiters and preserve source labels. Do not infer scoped instructions from arbitrary shell commands.
- Existing compaction and memo specs remain local requirements; this source review does not claim upstream provides matching implementations.

## License and provenance

The repository is [MIT licensed](https://github.com/deepseek-ai/deepseek-harness/blob/b2e3b2a0125854567a4a5fcba75782e42fe84901/LICENSE), Copyright (c) 2026 DeepSeek. If code or tests are copied or substantially adapted, the owning port must preserve the complete copyright, permission and disclaimer notice in a package-local `UPSTREAM_LICENSE` and identify source revision/paths in a short `UPSTREAM.md`. Those files ship with the adapted package. A conceptual reimplementation still cites its reference, but no source files or license text have been copied by this planning task.

Do not bring over vendored dependencies without checking their separate terms in [THIRD_PARTY_NOTICES.md](https://github.com/deepseek-ai/deepseek-harness/blob/b2e3b2a0125854567a4a5fcba75782e42fe84901/THIRD_PARTY_NOTICES.md). Excluded: Cordis runtime, schema DSL, JS SDK, provider stack, UI/diff cards, PTC runtime, agent teams, background jobs, persistent shells and platform sandbox implementations. Existing glue/router/sessions are reused instead.

## Status

Source inspection only. No implementation, clone, source-copy or upstream/local test execution is claimed. Acceptance commands in the individual specifications are for future implementers. Plan validation is separate from implementation verification.

Pinned provenance: [[deepseek-harness-port-source]].
