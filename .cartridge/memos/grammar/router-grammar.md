---
kind: grammar
description: "the model router cartridge — providers, credentials, catalog, launch — distinct from the external glue plugin of the same name"
overloads: the router cartridge, the model routing concept, the external glue plugin named router
date: "2026-09-12"
---

# router

Three senses. The **router** cartridge (`builtin/router`) owns providers,
credentials, the catalog, ranking, recovery and launch; its credentials and
state use the required `config_dir` and `data_dir`. **Model routing** is what
it does — a request comes in, the router picks the model — so "the router"
in prose means the cartridge doing that job. And outside cartridge, the glue
project's plugin is also called **router** — a different codebase that
happens to share the word; cartridge's record does not govern it. Authority is
`builtin/router` and its README.

The bite: inside this tree "router" is always the cartridge; the external
plugin only appears in cross-project notes. Do not read model-routing prose
as the proxy's work — the HTTP proxy is a separate surface; see
[[proxy-grammar]].