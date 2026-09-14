-- The profile of this checkout: every command loads these cartridges; which
-- one is used is the command (`run tui`, `mcp`, `launch`). `inject` adds keys
-- to an entry's needs; `inject = {"tool.*"}` names every tool key another entry
-- provides.
return {
	{ id = "auth", path = "auth" },
	{ id = "memo", path = "memo", inject = { "tool.*", "context.*", "source.*" } },
	{ id = "sessions", path = "sessions" },
	{ id = "docs", path = "docs" },
	{ id = "router", path = "router" },
	{ id = "policy", path = "policy" },
	{ id = "memory", path = "memory" },
	{ id = "gitfs", path = "gitfs", inject = { "sessions", "router" } },
	{ id = "fs", path = "fs" },
	{ id = "pty", path = "pty" },
	{ id = "prd", path = "prd" },
	{ id = "tools", path = "tools" },
	{ id = "live-record", path = "live/record" },
	{ id = "harness", path = "harness", inject = { "environment", "pty", "memory" }, config = { environment = "environment", terminal = "pty", memory = "memory" } },
	{ id = "agent", path = "agent", inject = { "tool.*" } },
	{ id = "live", path = "live" },
	{ id = "live-mcp", path = "live/mcp" },
	{ id = "mcp", path = "mcp", inject = { "tool.*", "policy.explain" } },
	{ id = "proxy", path = "proxy", inject = { "tool.*" } },
	{ id = "tui", path = "tui", inject = { "agent", "sessions", "buffers", "router", "pty" }, config = { bridge = true } },
}
