return {
	sessions = { dir = ".cartridge/sessions" },
	policy = { default = "ask" },
	router = {
		-- Each foreground chat owns a router; let concurrent chats coexist.
		-- The router passes its assigned address to launched clients.
		listen = { "127.0.0.1:0" },
		config_dir = ".cartridge/credentials",
		data_dir = ".cartridge/router",
	},
	harness = {
		-- Conservative request budget with reserved output headroom and
		-- automatic compaction through the router when a projection exceeds it.
		max_bytes = 240 * 1024,
		output_headroom = 16 * 1024,
		keep_turns = 4,
		summary_timeout_secs = 60,
		-- The turn ring: finished turns keep their files for hard reference,
		-- and `{op:"ring"}` distills older ones into the memory bank before
		-- dropping them. One sweep is bounded, so a long backlog drains over
		-- several sweeps instead of stalling a dispatch.
		ring_keep = 64,
		ring_sweep = 4,
	},

	memo = {
		max_output_bytes = 16 * 1024 * 1024,
	},
	gitfs = {
		-- Session overlay blobs and runtime state; gitignored, stays out of the tree.
		store_dir = ".cartridge/gitfs",
		ship = {
			-- Ship commits the session's owned paths on the current branch and
			-- pushes. No remote configured -> the commit stays local and the
			-- push is reported failed. Set gate_model to enable the LLM
			-- ship-or-hold gate (fails open); unset -> plain file-summary subject.
			push = true,
			remote = "origin",
			gate_timeout_ms = 30000,
			force_secrets = false,
		},
	},

	-- Durable knowledge bank; the memory-tool adapter exposes query/ingest
	-- to the agent as tool.memory. Endpoints default to a local Ollama.
	memory = { dir = ".cartridge/memory" },
	agent = {
		-- Configured model choice and request/step/output limits with safe
		-- defaults; approvals time out rather than hang.
		approval_timeout_secs = 300,
		cancel_timeout_secs = 5,
		max_prompt_bytes = 256 * 1024,
		max_steps = 50,
		max_tool_calls = 100,
	},
}
