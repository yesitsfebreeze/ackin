---
kind: decision
description: "Loadable plugins keep a JSON manifest and Lua entry in their own folder"
status: accepted
date: "2026-09-10"
uses:
  - usage: "[[read-usage]]"
    when: ["adding a plugin or resolving a profile path"]
---

# plugin-folder-manifests

## Decision

Each extension is called a cartridge and lives under `builtin/<name>/` with `cartridge.json` declaring `name` and `entry`. The entry is a Lua file relative to the folder and returns the existing component table or process descriptor. Profile paths select folders or their `cartridge.json` directly; profile IDs still identify configured instances independently of the component name. Explicit Lua file paths remain supported for programmatic composition and fixtures. `builtin/tui` and repository tooling are cartridges too; see [[tui-and-tools-are-cartridges]].

## Why

The user requested one folder per extension, named cartridges, with cartridge.json declaring its identity and entry. Keeping wrappers beside their Rust sources removes the split between top-level Lua files and plugin directories. JSON is parsed without executing plugin code and needs no new dependency.

## Consequences

The loader rejects missing or malformed manifests and entries escaping the folder. It watches the manifest, resolved Lua entry, and process executable. Invalid replacements preserve the active instance; disabled instances do not evaluate their entry. Commands beginning with `./` resolve beside the Lua entry; other commands retain working-directory/PATH resolution. Process working directories are unchanged. `just bundle` builds `dist/glue/` with the executables, runtime cartridge folders, and default profile. Glue falls back to builtin cartridges and profiles beside its executable when local/user paths are absent. The default profile and its offline integration fixture use folder paths. The schema and layout are documented in `builtin/README.md`.
