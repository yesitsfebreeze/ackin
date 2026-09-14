-- The one user profile. There is no `<profile>` layer under `.cartridge/`:
-- this file is the composition every command loads, and `config.lua` beside it
-- is the configuration every command reads. A command picks an entry point out
-- of this one host — `run tui` the terminal, `mcp` the stdio server, `launch`
-- the proxy — and never a different list of cartridges.
--
-- The profile composes by listing cartridges. `inject = {"tool.*"}` on an entry
-- gives it every tool key the entries below provide, and that same set is its
-- dispatch list, so a model request for any other name still fails with a
-- recorded unknown-tool error. Adding or removing a cartridge here changes the
-- tool surface of every front end at once; there is no second list of keys to
-- keep in step with it, and the UI palette asks the agent rather than being
-- told separately.
return {
	-- Path entries only: every component is independently replaceable by
	-- changing its manifest, Lua entry or profile entry, without rebuilding
	-- cartridge. The record and the fabric over it: `tool.*` is what the
	-- fabric graphs, so a cartridge added here is a tool the fabric
	-- indexes.

	-- Services the front ends share.
	{ id = "auth", path = "auth" },
	-- `context.*` and `source.*` are memo's own contracts: fs, memory and prd
	-- provide them, and memo learns which exist from this injection alone.
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

	-- The front ends. All of them are composed; which one runs is the command,
	-- not the profile. Nothing here binds a port on its own: the proxy listens
	-- only when a key names it (see `config.lua`), and the router and the live
	-- server take whatever port the machine hands them, so two hosts of this
	-- one profile coexist instead of fighting over an address. Live is a
	-- service now, not a surface: it has no page, and the terminal is the
	-- only thing that talks to it.
	{ id = "live", path = "live" },
	{ id = "live-mcp", path = "live/mcp" },
	-- `policy.explain` is optional to mcp: injected here, it can explain a
	-- refusal; absent, the refusal is bare. It is the profile that says so.
	{ id = "mcp", path = "mcp", inject = { "tool.*", "policy.explain" } },
	{ id = "proxy", path = "proxy", inject = { "tool.*" } },
	-- The one surface: `cartridge run tui` (terminal). Its chat backend wraps
	-- these injected services and asks `agent` for the palette's action list.
	-- It does not inject `live`: that edge deadlocks startup, and the terminal
	-- reaches the voice conversation over Live's own HTTP surface, using the
	-- address the launcher hands its attachment.
	{ id = "tui", path = "tui", inject = { "agent", "sessions", "buffers", "router", "pty" }, config = { bridge = true } },
}
