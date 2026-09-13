-- The proxy grants the tools the entries below provide: `tool.*` expands to
-- their keys and is also the proxy's grant list. Incoming requests cannot reach
-- additional host services or select a different workspace.
return {
	-- The record and the landscape over it: `tool.*` is what the landscape
	-- graphs, so a cartridge added here is a tool the landscape indexes.
	{ id = "memo", path = "memo", inject = { "tool.*", "memory" } },
	{ id = "sessions", path = "sessions" },
	{ id = "docs", path = "docs" },
	{ id = "router", path = "router" },
	{ id = "policy", path = "policy" },
	{ id = "harness", path = "harness" },
	{ id = "pty", path = "pty" },
	{ id = "memory", path = "memory" },
	{ id = "proxy", path = "proxy", inject = { "tool.*" } },
}
