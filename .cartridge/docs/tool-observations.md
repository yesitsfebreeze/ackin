Native tool observations reuse the existing runtime diagnostic file. Set
`CARTRIDGE_DIAGNOSTICS=/host/owned/diagnostics.jsonl` and
`CARTRIDGE_TOOL_OBSERVATIONS=1` to enable them. Both are disabled by default.
`CARTRIDGE_DIAGNOSTICS_MAX_BYTES` retains the existing 8 MiB default per file,
with one `.1` rotation. No sessions transcript, Memory service or second journal
is created. Existing non-observation fault diagnostics retain their own behavior.

`CARTRIDGE_TOOL_ACTORS` optionally contains a host-owned JSON object such as
`{"agent":{"actor":"agent","activity":"deliberate"},"panel":{"actor":"ui","activity":"read"},"poller":{"actor":"background","activity":"poll"}}`.
The keys are the runtime source names already used for diagnostics (for a Lua
process wrapper this is its source name, typically the file stem, **not** an
arbitrary profile entry ID). Configuration is frozen on first observation.
At most 128 sources and 16 KiB of configuration are accepted. Invalid entries
and unknown sources produce null attribution. This classifies host-configured
cartridges, not authenticated end users. Request arguments, model metadata and
caller-provided context never assign source/actor authority. The host must give
sources distinct names if it needs distinct attribution.

Coverage is the native cartridge `call` frame boundary for bounded `tool.*`
keys. Direct Lua, socket and nested SDK calls not crossing this boundary are
not automatically attributed. `describe` is discovery; `cancel` is a
cancellation request. Other activity comes only from the host map.

Each observed request emits a metadata-only `tool_attempt` before resolution
and a `tool_completion` if a result or interrupted future is observed. The host
mints an independent `observation_id`. Fields include source, actor/activity,
operation, dispatch, outcome, completion_known, elapsed milliseconds and serialized
response JSON bytes. Size is not tokens or billing. Raw arguments, responses,
error text and descriptor text are not copied into these records.

A valid tool error is distinct from transport failure or malformed response.
Cancellation acknowledgment does not establish that an earlier effect stopped.
`interrupted-outcome-unknown` and dropped futures preserve unknown completion.
An orphan attempt after abrupt process death/rotation does not prove that no
external effect happened. No observer failure retries or changes a tool result.
No telemetry completeness or rollback guarantee is made.

`provider_generation` names the process-local provider version captured under
the intended Service's reload gate. Nested calls cannot overwrite it. Explicit
reload-resume spanning generations clears the single-generation claim.
`descriptor_revision` is SHA-256 of a successful actual descriptor reply, cached
for the same tool and provider generation; it is **last observed**, not proof of
the current deployment. Both direct descriptor objects and established wrapped
descriptor responses are accepted. Descriptors over 64 KiB or without name,
description and input_schema remain unknown. The cache retains at most 128 entries;
a new generation/cache eviction begins unknown. A changed successful describe
updates its hash even if the provider generation is unchanged.

The existing diagnostic sink is best effort. A disabled, unwritable or full
sink may lose observations while execution succeeds. Diagnostic writing retains
its existing filesystem latency; no observer RPC is performed. A reader must
report only the retained window, preserve missing dimensions and state that
unobserved/lost records are unknown. General native runtime services remain
host-trusted. No automatic opt-in or profile configuration changes are made.
