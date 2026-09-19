# Trace

Trace is the record of agent and runtime activity. Correlation identifiers join related work. Node event executions and published envelopes reach the declared `trace` writer through the authenticated host; ASP exposes the writer's durable records and calendar compaction. Diagnostics join this record and may also be written to the configured diagnostic stream.

`activity.rs` delivers node activity independently of Lua handlers. Its queue holds 1024 pending records and retries failed delivery. If it overflows, the next delivered record reports the number omitted in `dropped_before`; in-flight records are process-local. Sensitive structured fields are redacted before delivery.


Trace observations carry a stable Unix-millisecond timestamp in both the activity and writer envelope. Telemetry is redacted and bounded at queue admission and delivery: activities above 48 KiB retain operation, origin, event, timestamp and outcome summaries, with explicit truncation, original byte counts and SHA-256 digests of the redacted data. The complete payload is not retained in these summaries. This leaves room inside the writer's 64 KiB append limit. Permanent size or timestamp rejections are logged and counted as dropped before the next delivery, rather than blocking the queue with retries.

The same limit applies at direct `trace` event dispatch, including user/assistant exchanges that bypass the telemetry queue. Oversized append envelopes also carry `writer_truncation` with the original redacted envelope byte count and digest.

Finished node executions carry `metrics.duration_ms`, measured with a monotonic clock and including dispatch waiting time. Explicit response usage contributes unsigned `input_tokens`, `cached_input_tokens` and `output_tokens`; supplied measured metrics may additionally carry `retries` and `tool_calls`. Missing measurements remain absent. Known request context retains `task_id`, `run_id`, `request_id`, `parent_id` and `revision` in the same metrics object. An explicit verification record has `{passed: boolean, reference: nonempty string}`; the host never infers verification success from a successful call. Metrics use a strict field/type whitelist so token counts survive redaction without admitting arbitrary content or credentials. Oversize summaries retain diagnostics and metrics alongside the existing event and outcome evidence.

The size guard counts serialized bytes without allocating a serialized buffer for records that fit. Oversized evidence still receives a digest of its complete redacted serialization; retry payloads remain stable.

Run the offline size-guard comparison with `cargo test --release --lib telemetry_size_guard_benchmark -- --ignored --nocapture`. It alternates baseline/current order over seven rounds and prints fixture name, round, iterations, baseline nanoseconds and current nanoseconds. The fixtures cover small, near-limit and oversized activity. This measures telemetry serialization CPU cost, not whole-task latency; oversized records include redacted evidence hashing and may have different tradeoffs from ordinary traffic.
