# Runtime profiles

Profiles compose the sibling cartridge repositories through `builtin/` links.
Configuration files contain paths and defaults; credentials and runtime stores remain local.

Runtime diagnostics are off by default. Set `CARTRIDGE_DIAGNOSTICS=stderr` or
set it to a file path to enable them; `CARTRIDGE_DIAGNOSTICS_MAX_BYTES` caps
that file with one rotated generation.

The agent runs up to eight independent read tools concurrently. Configure
`max_parallel_tools` (1–64) to change this. Mutations and shell calls retain
model order. Automatic usage journals are opt-in through `record_usage = true`
in each agent, MCP, or proxy configuration. Session checkpoints and explicit
memo/memory writes remain durable.

Event streams retain at most 256 events and 1 MiB per channel. Runtime and SDK
subscriber queues are bounded; lagging consumers receive an error and must
resynchronize from durable state. Replay cursors older than retained history
receive an explicit gap event.
