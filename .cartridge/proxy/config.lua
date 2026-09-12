return {
	sessions = { dir = ".cartridge/proxy-sessions" },
	memory = { dir = ".cartridge/memory" },
	router = {
		listen = { "127.0.0.1:4141" },
		config_dir = ".cartridge/credentials",
		data_dir = ".cartridge/router",
	},
	-- The launched agent owns its history and its own compaction, so this is
	-- not cartridge's conservative journal budget: it only has to stay under what
	-- the chosen route can take, and the router already filters routes by
	-- context. The proxy caps the same request as a resource guard.
	harness = { max_bytes = 8 * 1024 * 1024, output_headroom = 64 * 1024 },
	-- This endpoint has no interactive approver, so an explicit allowlist is
	-- the approval channel: the human who launched the agent approves writes
	-- on the client side (its own permission prompts), and the keys listed
	-- here execute without a cartridge-side prompt. Every granted dispatch still
	-- lands in the observation journal under its caller and turn; anything
	-- not listed stays at `ask` and is denied with the operation named.
	policy = { default = "ask", tools = { memo = "allow", shell = "allow" } },
	proxy = {
		listen = "127.0.0.1:4142",
		key_env = "CARTRIDGE_PROXY_KEY",
		cwd = ".",
		max_steps = 50,
		max_tool_calls = 100,
		timeout_secs = 300,
		max_bytes = 8 * 1024 * 1024,
	},
}
