# Trace

Trace is the record of agent and runtime activity. Correlation identifiers join related work. Node event executions and published envelopes reach the declared `trace` writer through the authenticated host; ASP exposes the writer's durable records and calendar compaction. Diagnostics join this record and may also be written to the configured diagnostic stream.

`activity.rs` delivers node activity independently of Lua handlers. Its queue holds 1024 pending records and retries failed delivery. If it overflows, the next delivered record reports the number omitted in `dropped_before`; in-flight records are process-local. Sensitive structured fields are redacted before delivery.


Trace observations carry a stable Unix-millisecond timestamp in both the activity and writer envelope. Telemetry is redacted and bounded at queue admission and delivery: activities above 48 KiB retain operation, origin, event, timestamp and outcome summaries, with explicit truncation, original byte counts and SHA-256 digests of the redacted data. The complete payload is not retained in these summaries. This leaves room inside the writer's 64 KiB append limit. Permanent size or timestamp rejections are logged and counted as dropped before the next delivery, rather than blocking the queue with retries.

The same limit applies at direct `trace` event dispatch, including user/assistant exchanges that bypass the telemetry queue. Oversized append envelopes also carry `writer_truncation` with the original redacted envelope byte count and digest.
