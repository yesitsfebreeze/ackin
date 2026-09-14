-- The one user profile's configuration. Every command reads this file; where a
-- surface needs its own answer, the answer comes from the environment the
-- command already sets, not from a second profile directory.
--
-- This file names what is *this project's*: where things are stored, which
-- addresses are bound, which model answers. It does not restate limits. Every
-- cap, timeout and budget is declared with its cartridge in that cartridge's
-- `cartridge.json`, and `cartridge settings` prints all of them with what they
-- are set to and which file settled it. Pinning one here is how it quietly
-- stops following the declaration when the declaration is raised, which is
-- what this file used to do to `agent.max_steps`, `proxy.max_bytes` and the
-- rest of the numbers that have since moved next to the code that spends them.
--
-- Three layers, each laid over the last, field by field:
--   the declaration in cartridge.json   defaults, bounds and documentation
--   ~/.cartridge/config.lua             this machine, across projects
--   this file                           this project
-- `cartridge settings --template` prints every key at its current value in
-- this shape, ready to save as either file.

-- The harness proxy listens only when a key exists to authenticate it.
-- `cartridge launch` mints one before the host loads and takes whatever
-- loopback port the machine gives it, so two launched agents never collide;
-- `just proxy <port>` sets both key and a fixed address. Every other command
-- leaves the address unbound, so an MCP server or a terminal host never
-- binds a listener nothing will authenticate against.
local proxy_listen = os.getenv("CARTRIDGE_PROXY_LISTEN")
if proxy_listen == nil or proxy_listen == "" then
	local key = os.getenv("CARTRIDGE_PROXY_KEY")
	proxy_listen = (key ~= nil and key ~= "") and "127.0.0.1:0" or nil
end

return {
	-- One session store for one profile. Turns, journals and buffers are the
	-- host's, not the front end's, so a conversation started in the terminal is
	-- the same record an MCP client or a launched agent reads.
	sessions = { dir = ".cartridge/sessions" },

	-- Allow by default: a tool is on unless something here turns it off. The
	-- policy cartridge knows no tool and ships no built-in decision, so every
	-- per-tool rule lives here, on the composition, and a tool not named here
	-- is governed by `default` alone. The explicit rows below are the tools
	-- this project used to prompt for; they are kept so the list is visible.
	-- Approval, where it happens at all, happens at the client: the MCP client
	-- prompts its own user, and a launched agent prompts through its own
	-- permission surface. Every granted dispatch still lands in the observation
	-- journal under its caller and turn.
	policy = {
		default = "allow",
		tools = {
			read = "allow", glob = "allow", grep = "allow", search = "allow", docs = "allow",
			write = "allow", edit = "allow", shell = "allow",
			memo = "allow", memory = "allow", gitfs = "allow", ship = "allow",
			fs = "allow", sessions = "allow", prd = "allow",
			live = "allow", live_note = "allow", live_work = "allow", live_agent = "allow",
		},
	},

	router = {
		-- Each foreground chat owns a router; let concurrent chats coexist.
		-- The router passes its assigned address to launched clients.
		listen = { "127.0.0.1:0" },
		config_dir = ".cartridge/credentials",
		data_dir = ".cartridge/router",
	},
	auth = { credentials_dir = ".cartridge/credentials", router_data_dir = ".cartridge/router" },

	-- This project's request budget, pinned under the declared default on
	-- purpose. Two consumers read it: cartridge's own context op compacts when
	-- it exceeds this, and the proxy treats it as the point past which it stops
	-- adding retrieved context to a launched agent's request. Neither refuses on
	-- it -- proxy.max_bytes is the only wall -- so the cost of the pin is that a
	-- conversation past it keeps going without recall. Being a pin, it stops
	-- following `harness/cartridge.json` when that declaration is raised.
	harness = {
		max_bytes = 1024 * 1024,
		output_headroom = 64 * 1024,
	},

	-- Session overlay blobs and runtime state; gitignored, stays out of the tree.
	gitfs = { store_dir = ".cartridge/gitfs" },

	-- Durable knowledge bank; the cartridge exposes query/ingest to the agent
	-- as tool.memory itself. Endpoints default to a local Ollama.
	memory = { dir = ".cartridge/memory" },
	prd = { default_board = "root" },

	-- The voice service the terminal attaches to. Port 0: the machine assigns
	-- it and the launcher reads the address back off `call live {op:"status"}`,
	-- so a second host of this profile does not collide with the first.
	live = {
		port = tonumber(os.getenv("CARTRIDGE_LIVE_PORT") or "0"),
		dir = ".cartridge/live-data",
		cwd = "..",
		model = "gpt-live-1",
		voice = "marin",
	},
	["live-record"] = { dir = ".cartridge/live-data" },

	mcp = { cwd = ".." },
	proxy = {
		listen = proxy_listen,
		key_env = "CARTRIDGE_PROXY_KEY",
		cwd = ".",
	},
}
