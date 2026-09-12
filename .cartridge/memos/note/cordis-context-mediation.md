---
kind: note
description: "Cordis enforces declared dependencies at the point of access through a Proxy, isolates bindings by realm symbol, and adjusts use through interception — with ctx.get and root access as escape hatches"
date: "2026-09-09"
uses:
  - usage: "[[read-usage]]"
    when: ["asking how Cordis scopes what a plugin can see, or whether it is a security boundary"]
    tags: [research]
---

# cordis-context-mediation

Paper §5.1.4, Algorithm 6: property access `ctx[key]` goes through a Proxy
that walks the fiber chain upward and returns the first committed binding;
reaching a fiber that declares the key without having committed it throws
(inactive access), reaching the root without a declaration throws
(undeclared access). `ctx.get(key)` is the bare store lookup that never
fails. §5.1.2: `isolate(key, realm)` derives a child context whose realm
table maps the key to a fresh symbol, so two contexts resolve independent
bindings; `intercept(key, metadata)` merges metadata consulted when the
binding is used, changing how, not whether, it resolves — so it can change at
runtime without a reload. §6.3 calls this capability-style access control and
says plainly that sandboxing untrusted code needs an execution boundary
outside the language.

Source ([[cordis-repo]]): the proxy enforces injected, inherited, or
self-provided access from the definition-site fiber
(https://github.com/cordiverse/cordis/blob/f8ea3cd50f1a5724e8e715995bcde131c9c12b2c/packages/core/src/reflect.ts#L61-L99); `isolate` keys the store by
symbol, and a shared symbol reunites branches
(https://github.com/cordiverse/cordis/blob/f8ea3cd50f1a5724e8e715995bcde131c9c12b2c/packages/core/src/context.ts#L65-L76). Service proxies carry both a
definition site (what the implementation may see) and a use site (who owns
the effects it registers) (https://github.com/cordiverse/cordis/blob/f8ea3cd50f1a5724e8e715995bcde131c9c12b2c/packages/core/src/utils.ts#L110-L215). Plain
`ctx.emit` is not filtered by branch; a receiver's `Context.filter` is.

What would change this: the escape hatches being removed or gated.
Sibling: [[cordis-reactive-coeffects]].
