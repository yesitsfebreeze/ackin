//! The ask: every subcommand and its arguments, as clap reads them.

use std::path::PathBuf;

use cartridge::{Error, Result};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
	name = "cartridge",
	about = "cartridges on a socket: run in the back, handle the events",
	// `help` is a subcommand of ours: the manual of what is composed, not clap's usage text.
	disable_help_subcommand = true
)]
pub(crate) struct Cli {
	/// Directory containing bundled cartridges (each with cartridge.json)
	#[arg(long, global = true)]
	pub(crate) dir: Option<PathBuf>,
	/// Automatic execution: bypass tool policy and skip resolver provenance recording
	#[arg(long, global = true)]
	pub(crate) yolo: bool,
	#[command(subcommand)]
	pub(crate) command: Command,
}

#[derive(Subcommand)]
pub(crate) enum Command {
	Daemon,
	/// Start the harness proxy and run an agent against it: `cartridge launch claude -- -p hi`
	Launch {
		agent: String,
		#[arg(long, default_value = "auto:code")]
		model: String,
		#[arg(trailing_var_arg = true, allow_hyphen_values = true)]
		args: Vec<String>,
	},
	/// Serve the profile's tools to an MCP client over this terminal's stdio:
	/// `claude mcp add cartridge -- cartridge mcp`
	Mcp,
	/// Load a profile, call one service in the foreground, and dispose it
	Run {
		key: String,
		#[arg(default_value = "null")]
		args: String,
	},
	Send {
		name: String,
		#[arg(default_value = "null")]
		data: String,
	},
	/// Publish one event on a stream channel
	Publish {
		channel: String,
		#[arg(default_value = "null")]
		data: String,
	},
	/// Subscribe to a stream channel and print every event on it as it arrives
	Follow {
		channel: String,
	},
	Tail,
	/// Call a provided key with one JSON argument and print the reply
	Call {
		key: String,
		#[arg(default_value = "null")]
		args: String,
		/// Tag this call with a trace id; one is minted when absent
		#[arg(long)]
		trace: Option<String>,
	},
	Reload,
	Status,
	/// Enter, leave or report debug mode on the running host: `cartridge debug on`
	Debug {
		#[arg(default_value = "status", value_parser = ["on", "off", "status"])]
		state: String,
	},
	Socket,
	/// Unlink the socket files no listener answers on, left by runtimes that
	/// were killed rather than stopped
	Sweep,
	/// Cartridges of the profile and what each one needs, resolved to its provider
	List,
	/// Every cartridge installed under the cartridge root, and what each need binds to
	Ledger,
	/// The manual: every document the host and each cartridge carry, as one
	/// tree of modules, documents and sections. On a terminal a picker descends
	/// it; otherwise `cartridge help <id>/<file>#<section>` prints one node and
	/// `cartridge help <words>` finds every line holding them
	Help {
		/// An address (`memo`, `memo/README.md`, `memo/README.md#usage`) or words
		/// to search for. Absent is the list of modules
		what: Vec<String>,
		/// Print the addressed node, or the search hits, as JSON; alone, the whole manual
		#[arg(long)]
		json: bool,
	},
	/// Every tunable value this profile has: the host's own and each
	/// cartridge's, with what it is set to and which file settled it.
	/// Name a key or a cartridge to narrow it: `cartridge settings host`,
	/// `cartridge settings agent.max_steps`
	Settings {
		/// A cartridge id, or one dotted key under it. Absent lists everything
		what: Option<String>,
		/// Print the listing as JSON instead of a table
		#[arg(long)]
		json: bool,
		/// Print a commented `config.lua` carrying every key at its current
		/// value, ready to save as `~/.cartridge/config.lua` or the project's
		#[arg(long)]
		template: bool,
	},
	Up {
		key: String,
	},
	/// Launch one node of a chain and hand off: resolve it against the fresh
	/// ledger, read its program, and exec into it. A node coming up enters
	/// this, not a person
	Enter {
		node: String,
		/// The node paths still to launch after this one, in chain order
		#[arg(long, default_value = "[]")]
		rest: String,
	},
	/// Load the profile and run every contract its cartridges declare; name a
	/// cartridge to verify just that one, in isolation, against its own contract
	Verify {
		/// The cartridge: a ledger path from the cartridge root, or the folder
		/// it sits in
		cartridge: Option<String>,
	},
	/// Run one chain node of a Lua-entry cartridge in-host. The re-entry
	/// execs into this, so a node's program is the host itself; a person
	/// never invokes it, and the ledger's env protocol is the only way in
	#[command(hide = true)]
	Node,
}

impl Cli {
	/// What clap cannot say: a flag that only some commands take.
	pub(crate) fn check(&self) -> Result<()> {
		if self.yolo && !matches!(&self.command, Command::Run { .. } | Command::Daemon) {
			return Err(Error::Argument(
				"--yolo requires run or daemon; it cannot change an existing daemon".into(),
			));
		}
		Ok(())
	}
}
