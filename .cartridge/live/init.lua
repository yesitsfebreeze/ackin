-- The same local agent and tool composition, with a browser voice surface.
-- Wildcard injection gives the orchestrator every tool in this profile.
return {
  { id = "auth", path = "auth" },
  { id = "live-record", path = "live/record" },
  { id = "memo", path = "memo", inject = { "tool.*" } },
  { id = "sessions", path = "sessions" },
  { id = "docs", path = "docs" },
  { id = "router", path = "router" },
  { id = "gitfs", path = "gitfs", inject = { "sessions" } },
  { id = "fs", path = "fs" },
  { id = "policy", path = "policy" },
  { id = "memory", path = "memory" },
  { id = "memory-tool", path = "memory-tool" },
  { id = "pty", path = "pty" },
  { id = "harness", path = "harness", inject = { "environment", "pty", "memory" }, config = { environment = "environment", terminal = "pty", memory = "memory" } },
  { id = "workspace", path = "workspace" },
  { id = "agent", path = "agent", inject = { "tool.*" } },
  { id = "live", path = "live" },
}
