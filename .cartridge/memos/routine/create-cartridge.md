---
kind: routine
description: Create and register a cartridge with one manifest, tested behavior, and documented commands
uses:
  - usage: "[[run-usage]]"
    when: [creating a cartridge, adding a service to the cartridge workspace]
---

# Create a cartridge

## Inputs

Run `cartridge help` from cartridge.ctg, then read `llms.txt`,
`docs/writing-good-modules.txt`, and `docs/creating-cartridges.txt` there.
Choose the repository name, provided services, required services, configuration,
resource ownership, and the observable behavior to test. Read the nearest
AGENTS.md; memory changes require an isolated branch and worktree.

## Do

1. Create the sibling repository. Keep `cartridge.json` and its Lua entry there.
   Declare behavior with `provide`, `needs`, configuration and any grants; include
   a purpose and structured run/test/build/check commands where applicable.
   Do not create a separate development JSON.
   Declare every tunable value in `settings`: one entry per dotted key with a
   type, a default, bounds and one line of documentation. The host fills and
   checks each one before `apply`, so the cartridge carries no fallback of its
   own and re-checks no bound the declaration already states — check only what a
   declaration cannot say, such as a cap being zero. Defaults are generous: a
   limit stops a runaway rather than rationing the machine. A free function or a
   support library that cannot read a configuration takes an installable
   `limits` module instead, declared as this cartridge's `limits.*` keys.
2. Implement Lua services using `ctx:provide`, or a Rust SDK process with a Lua
   adapter, a matching executable name, and consistent hello declarations.
   Call declared needs directly for required results and reach nothing else:
   an optional integration is an event the other side listens to, never a
   copy of its protocol in this cartridge. Publish useful progress,
   state-change, result, and failure events through the API that reaches the
   intended consumers; document payloads, timing, and delivery limits. A tool
   says in its descriptor (`reads`) which of its ops only read.
   Keep process stdout reserved for the protocol and dispose owned resources.
3. Register the parent submodule, relative builtin link, development catalog,
   and (for shared Rust packages) workspace member. Add only the profiles and
   configuration needed by the new service: `config.lua` carries what is this
   project's — paths, addresses, model choices — never a limit that restates its
   declared default. Run `cartridge settings <name>`; a non-zero exit means the
   cartridge is configured with a key it never declared. Extend module
   test/check dispatch when the helper does not yet support that module type.
4. Build and invoke the service through a real host. For a tool, verify its
   discovery schema and invocation under the intended MCP/profile policy.
5. Write `.cartridge/help.md` (title, what it owns, `## Use`, `## Read next`
   with links relative to the cartridge directory) and confirm
   `cartridge help <name>` prints it. Run `just isolation` from the composed
   repository and fix every reported line in the cartridge that owns it.
6. Discover guidance with memo `types`, `fabric`, `resolve`, and `read`.
   Document contracts, dependency rationale, and checks through memo `write`;
   carry `expected_revision` when editing a workspace memo. Shipped memos are
   read-only through the tool: edit their source and verify discovery with the
   cartridge enabled. Keep executable declarations in cartridge.json. Commit
   affected submodules before updating parent pointers.

## Check

Follow the complete hello example in the authoring guide: calling hello.echo
with `{"message":"hello"}` must return that JSON. Use selected module tests,
format/lint checks, ledger inspection, and real protocol checks appropriate to
the implementation. Assert failures for invalid input and test dependency and
resource lifecycle behavior. Use an isolated profile and private temporary state.
Verify a separate intended consumer receives the documented events, required
dependencies reach real results, and contract memos are discoverable.
All documented commands must resolve from the directory containing the manifest.
Use verify only for a declared, self-contained contract; invoke contracts that
need profile configuration through the corresponding profile.

## Failure

Stop at the failing boundary and inspect its manifest, profile, or protocol
error. Do not relax strict parsing, silently run another module's tests, loosen
policy, or stop unrelated daemons to make a check pass. Fix the declaration or
implementation, rerun the affected check, and record the observed correction.
Preserve stores, unrelated edits, and the old sys compatibility sessions.
