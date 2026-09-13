-- The profile composes by listing cartridges. `inject = {"tool.*"}` on the
-- agent gives it every tool key the entries below provide, and that same set is
-- its dispatch list, so a model request for any other name still fails with a
-- recorded unknown-tool error. Adding or removing a cartridge here changes the
-- agent's tools; there is no second list of keys to keep in step with it, and
-- the UI palette asks the agent rather than being told separately.
return {
	-- Path entries only: every component is independently replaceable by
	-- changing its manifest, Lua entry or profile entry, without rebuilding cartridge.
	-- The record and the landscape over it: `tool.*` is what the landscape
	-- graphs, so a cartridge added here is a tool the landscape indexes.
	{ id = "memo", path = "memo", inject = { "tool.*" } },
	{ id = "sessions", path = "sessions" },
	{ id = "docs", path = "docs" },
	{ id = "router", path = "router" },
	{ id = "gitfs", path = "gitfs" },
	{ id = "policy", path = "policy" },
	{ id = "memory", path = "memory" },
	{ id = "memory-tool", path = "memory-tool" },
	-- The bank is an entry-level injection like the shell scan: the harness
	-- distills evicted turns into it, and a profile without a bank omits both.
	{ id = "harness", path = "harness", inject = { "environment", "pty", "memory" }, config = { environment = "environment", terminal = "pty", memory = "memory" } },
	{ id = "agent", path = "agent", inject = { "tool.*" } },
	-- One UI cartridge: `cartridge run ui` (terminal). Its chat backend wraps these
	-- injected services and asks `agent` for the palette's action list.
	{ id = "pty", path = "pty" },
	{ id = "ui", path = "ui", inject = { "agent", "sessions", "buffers", "router", "pty" }, config = { bridge = true } },
}
