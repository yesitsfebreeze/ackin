-- The profile composes the tool surface by listing cartridges: `tool.*` injects
-- every tool key the entries below provide, and the mcp cartridge serves exactly
-- what it was injected. Adding a cartridge here adds its tools; there is no
-- second list of keys to keep in step with it.
return {
	-- The record and the landscape over it: `tool.*` is what the landscape
	-- graphs, so a cartridge added here is a tool the landscape indexes.
	{ id = "memo", path = "memo", inject = { "tool.*" } },
	{ id = "sessions", path = "sessions" },
	{ id = "docs", path = "docs" },
	{ id = "policy", path = "policy" },
	{ id = "pty", path = "pty" },
	{ id = "gitfs", path = "gitfs", inject = { "sessions" } },
	{ id = "memory", path = "memory" },
	{ id = "memory-tool", path = "memory-tool" },
	{ id = "prd", path = "prd" },
	{ id = "mcp", path = "mcp", inject = { "tool.*" } },
}
