# Root and nested replacement

The profile loader and `ctx:cartridge` retain their existing ways to discover and
resolve components. Both use `Host::load_component` and the runtime context to
create nodes. Active replacements share `Host::replace_active` in `src/loader.rs`.
The profile adapter retains entry configuration, source stamps, and initial
failure recovery. The nested adapter retains its rebuild closure and the original
parent, isolation, and interception context.

The shared transaction prepares each participating process once, drains service
calls, stages the candidate in isolated provided-key realms, and publishes through
`Runtime::switch`. A successful switch updates the profile's current handle and
source stamps synchronously, before releasing the transaction or disposing the
old generation. A rejected candidate is disposed, prepared processes are cancelled,
and the old service values remain usable. Rejection does not replay the request.

`Host::replace_node(uid)` returns the current generation UID on success at either
depth. Callers can use that UID for the next replacement. Failed active candidates
return an error at either depth; the root adapter also retains its diagnostic
report. A corrected initially failed profile entry follows ordinary asynchronous
startup, returning its newly spawned current handle; readiness remains separately
observable. Replacement of an active generation waits for a valid candidate.

The existing sharing rules are unchanged. Legacy Lua composition shares keys
unless the caller explicitly isolates them. Private child realms stay private,
and nesting does not instantiate another copy of a shared router or PTY provider.
The parameterized `composition` test checks the same public operation at root and
nested depth, including retained consumer references, private realms, intercepted
metadata, returned UIDs, refused migrations and changed provided keys, recovery,
source stamps, and one shared-provider application.

This change does not add memory banks, persistent child identities, automatic
child source watching, provider activation, or a launch policy. Those are separate
contracts; the historical nested-entry work record is not declared complete here.
