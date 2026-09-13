return {
	sessions = { dir = ".cartridge/mcp-sessions" },
	memory = { dir = ".cartridge/memory" },
	-- The tool set follows the profile now, so the policy is what bounds this
	-- server, not the entry list: a cartridge added above is exposed, but its
	-- tools answer `ask` until a rule here says otherwise. The MCP client is the
	-- approval surface — it prompts its own user before every call — and `memo`
	-- is allowed on that basis. `tool.shell` reaches the user's own shell and
	-- `tool.gitfs`/`tool.ship` write the working tree; those stay at the
	-- policy's `ask` until someone decides otherwise here.
	policy = { tools = { memo = "allow" } },
	mcp = { cwd = ".." },
}
