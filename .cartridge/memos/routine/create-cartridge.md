---
kind: routine
description: Create and register a cartridge with one manifest, tested behavior, and documented commands
uses:
  - usage: "[[run-usage]]"
    when: [creating a cartridge, adding a service to the cartridge workspace]
---

# Create a cartridge

## Inputs

Read `llms.txt` and `docs/llms/creating-cartridges.txt` in cartridge.ctg.
Choose the repository name, provided services, required services, configuration,
resource ownership, and the observable behavior to test. Read the nearest
AGENTS.md; memory changes require an isolated branch and worktree.

## Do

1. Create the sibling repository. Keep `cartridge.json` and its Lua entry there.
   Declare behavior with `provide`, `needs`, configuration and any grants; include
   a purpose and structured run/test/build/check commands where applicable.
   Do not create a separate development JSON.
2. Implement Lua services using `ctx:provide`, or a Rust SDK process with a Lua
   adapter, a matching executable name, and consistent hello declarations.
   Keep process stdout reserved for the protocol and dispose owned resources.
3. Register the parent submodule, relative builtin link, development catalog,
   and (for shared Rust packages) workspace member. Add only the profiles and
   configuration needed by the new service. Extend module test/check dispatch
   when the helper does not yet support that module type.
4. Build and invoke the service through a real host. For a tool, verify its
   discovery schema and invocation under the intended MCP/profile policy.
5. Document context and decisions in memos; keep executable declarations in
   cartridge.json. Commit affected submodules before updating parent pointers.

## Check

Follow the complete hello example in the authoring guide: calling hello.echo
with `{"message":"hello"}` must return that JSON. Use selected module tests,
format/lint checks, ledger inspection, and real protocol checks appropriate to
the implementation. Assert failures for invalid input and test dependency and
resource lifecycle behavior. Use an isolated profile and private temporary state.
All documented commands must resolve from the directory containing the manifest.
Use verify only for a declared, self-contained contract; invoke contracts that
need profile configuration through the corresponding profile.

## Failure

Stop at the failing boundary and inspect its manifest, profile, or protocol
error. Do not relax strict parsing, silently run another module's tests, loosen
policy, or stop unrelated daemons to make a check pass. Fix the declaration or
implementation, rerun the affected check, and record the observed correction.
Preserve stores, unrelated edits, and the old sys compatibility sessions.
