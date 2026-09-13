return {
  auth = { credentials_dir = ".cartridge/credentials", router_data_dir = ".cartridge/router" },
  ["live-record"] = { dir = ".cartridge/live-data" },
  live = { port = 4317, dir = ".cartridge/live-data", cwd = "..", graph_roots = { "../alois" }, model = "gpt-live-1", voice = "marin" },
  workspace = { dir = ".cartridge/workspace-pages" },
  sessions = { dir = ".cartridge/live-sessions" },
  router = { listen = { "127.0.0.1:0" }, config_dir = ".cartridge/credentials", data_dir = ".cartridge/router" },
  -- User-selected YOLO mode: the voice agent executes tools without approval prompts.
  policy = { default = "allow", tools = { read = "allow", glob = "allow", grep = "allow", search = "allow", write = "allow", edit = "allow", shell = "allow", memo = "allow", memory = "allow", gitfs = "allow", ship = "allow", live_note = "allow", live_work = "allow" } },
  memory = { dir = ".cartridge/memory" },
  memo = { yolo = true, max_output_bytes = 16 * 1024 * 1024 },
  gitfs = { store_dir = ".cartridge/gitfs" },
  harness = { max_bytes = 4 * 1024 * 1024, output_headroom = 16 * 1024, keep_turns = 4, summary_timeout_secs = 60, ring_keep = 64, ring_sweep = 4 },
  agent = { yolo = true, approval_timeout_secs = 300, cancel_timeout_secs = 5, max_prompt_bytes = 4 * 1024 * 1024, max_steps = 50, max_tool_calls = 100 },
}
