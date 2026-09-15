use std::path::PathBuf;

use cartridge::{Error, Result};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
	name = "cartridge",
	about = "a host for cartridges: programs wired together over local sockets",
	// `help` is a subcommand of ours: the manual of what is composed, not clap's usage text.
	disable_help_subcommand = true
)]
pub(crate) struct Cli {
	/// Cartridge root: entry paths in init.lua resolve against it (default: the project)
	#[arg(long, global = true)]
	pub(crate) dir: Option<PathBuf>,
	/// Automatic execution: trust the project's files, bypass tool policy and
	/// skip resolver provenance recording — one confirmation, Enter accepts
	#[arg(long, global = true)]
	pub(crate) yolo: bool,
	#[command(subcommand)]
	pub(crate) command: Command,
}

#[derive(Subcommand)]
pub(crate) enum Command {
	/// Make the working directory a project: choose which of the cartridges
	/// found take part, link or clone them under the cartridge root, and write
	/// the `.cartridge/init.lua` that names them. Asks on a terminal
	Setup {
		/// A directory holding cartridge folders to choose from; repeatable.
		/// Absent: the cartridge root, or the terminal is asked
		#[arg(long)]
		from: Vec<PathBuf>,
		/// Take these cartridges by name, without asking
		#[arg(long, value_delimiter = ',')]
		with: Vec<String>,
		/// Take every cartridge offered, without asking
		#[arg(long)]
		yes: bool,
		/// The catalog of known repositories to offer; absent,
		/// `~/.cartridge/catalog.json` (or `$CARTRIDGE_HOME/catalog.json`)
		#[arg(long)]
		catalog: Option<PathBuf>,
	},
	/// Ask every composed cartridge that declares a `doctor` event whether it
	/// is healthy here, and say which are not
	Doctor,
	/// Start the descriptor and serve the host socket until stopped
	Daemon,
	/// Start the harness proxy and run an agent against it: `cartridge launch claude -- -p hi`
	Launch {
		agent: String,
		#[arg(long, default_value = "auto:code")]
		model: String,
		#[arg(trailing_var_arg = true, allow_hyphen_values = true)]
		args: Vec<String>,
	},
	/// Serve the descriptor's tools to an MCP client over this terminal's stdio:
	/// `claude mcp add cartridge -- cartridge mcp`
	Mcp,
	/// Start the descriptor, send one event, print the first answer, and stop
	Run {
		event: String,
		#[arg(default_value = "null")]
		data: String,
	},
	/// Send an event on the running host and print the first answer
	Call {
		event: String,
		#[arg(default_value = "null")]
		data: String,
	},
	/// Send an event on the running host and print every listener's answer
	Send {
		event: String,
		#[arg(default_value = "null")]
		data: String,
	},
	/// Print every event on a channel: `lifecycle`, or `<cartridge>.<channel>`
	Follow {
		channel: String,
		/// Replay retained events after this sequence number first
		#[arg(long)]
		since: Option<u64>,
	},
	/// Every cartridge of the running host and its state
	Status,
	/// Reload the descriptor, or restart one cartridge
	Reload { cartridge: Option<String> },
	/// Stop the running host
	Stop,
	/// The host socket of this project
	Socket,
	/// Cartridges of the descriptor and what each one needs, resolved to its provider
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
	/// Every tunable value this descriptor has, with what it is set to and which
	/// file settled it: `cartridge settings host`, `cartridge settings agent.max_steps`
	Settings {
		/// A cartridge id, or one dotted key under it. Absent lists everything
		what: Option<String>,
		/// Print the listing as JSON instead of a table
		#[arg(long)]
		json: bool,
		/// Print a commented `config.lua` carrying every key at its current value
		#[arg(long)]
		template: bool,
	},
	/// Record the SHA-256 of every `*.lua` and `cartridge.json` under a project
	/// or cartridge folder, so this machine runs it. Without a path, this project
	Trust {
		/// The directory; absent is the project of this working directory
		path: Option<PathBuf>,
		/// Forget a directory instead of recording it
		#[arg(long)]
		revoke: bool,
		/// Print every directory this machine trusts
		#[arg(long)]
		list: bool,
		/// Ask on the terminal before recording, and only when something is untrusted
		#[arg(long)]
		ask: bool,
	},
	/// Start the descriptor and run every contract its cartridges declare; name a
	/// cartridge to verify just that one against its own contract
	Verify {
		/// A ledger path from the cartridge root, or the folder it sits in
		cartridge: Option<String>,
	},
	/// Run one cartridge's `init.lua` as a node of the base that started this process
	#[command(hide = true)]
	Node,
	/// Restrict this process to a compiled policy, then become the command after `--`
	#[command(hide = true, name = "__confine")]
	Confine {
		policy: String,
		#[arg(last = true, required = true, allow_hyphen_values = true, num_args = 1..)]
		command: Vec<String>,
	},
}

impl Cli {
	pub(crate) fn check(&self) -> Result<()> {
		if self.yolo
			&& !matches!(
				&self.command,
				Command::Run { .. } | Command::Daemon | Command::Launch { .. }
			) {
			return Err(Error::Argument(
				"--yolo requires run, launch or daemon; it cannot change an existing daemon".into(),
			));
		}
		Ok(())
	}
}
