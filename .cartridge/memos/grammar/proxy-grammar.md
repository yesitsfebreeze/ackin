---
kind: grammar
description: "two proxies: the harness request proxy cartridge and the router's HTTP proxy — different listeners, one word"
overloads: the proxy cartridge, the harness request proxy, the router's HTTP proxy
date: "2026-09-12"
---

# proxy

Three senses, two systems. The **proxy** cartridge (`builtin/proxy`) is the
harness request proxy: it injects the live system memos and tool schemas into
a raw model request, runs harness tools internally, and serves OpenAI- and
Anthropic-shaped endpoints on `127.0.0.1:4142` in the `proxy` profile. The
**router's HTTP proxy** is a different listener: the router cartridge
publishes an HTTP proxy on an OS-assigned loopback port in the default
profile, for outbound requests to providers. One word, opposite directions —
the harness proxy faces callers, the router's proxy faces providers.
Authority is `builtin/proxy/README.md` and [[router-grammar]].

The bite: "the proxy port" does not say which listener. The harness proxy is
opt-in via the `proxy` profile; the router's exists in every default run.
Whether one of them should take a different word is
[[streamline-the-overloaded-names]].