//! The command line, in the order one invocation runs it:
//!
//! 1. [`args`] — what was asked, as clap reads it;
//! 2. [`project`] — where it runs: the root, the profile, the settled settings;
//! 3. [`run`] — the one subcommand, dispatched to the function that is it:
//!    [`host`] composes a host in this process, [`client`] talks to a running
//!    one, [`node`] launches and links a chain, [`listing`], [`settings`] and
//!    [`manual`] read the composition and print it;
//! 4. the exit code, with the failure said once on stderr.
//!
//! What a command prints is its output. What it reports about itself goes
//! through `tracing`, on stderr at the level `CARTRIDGE_LOG` selects.

mod args;
mod client;
mod host;
mod listing;
mod manual;
mod node;
mod project;
mod settings;

use std::process::ExitCode;

use cartridge::{Error, Result};
use clap::Parser;
use serde_json::{json, Value};

use args::{Cli, Command};
pub(crate) use project::{exe, Project};

/// Exit codes: a failed command, and a command asked for wrongly.
pub(crate) const FAILED: u8 = 1;
pub(crate) const USAGE: u8 = 2;

/// Say why on stderr and answer with the exit code. The one place a command
/// prints its own failure, so every one reads the same.
pub(crate) fn fail(code: u8, why: impl std::fmt::Display) -> ExitCode {
	eprintln!("{why}");
	ExitCode::from(code)
}

/// The exit code a host error earns: a wrong ask is usage, anything else failed.
fn code_of(error: &Error) -> u8 {
	match error {
		Error::Argument(_) => USAGE,
		_ => FAILED,
	}
}

pub fn main() -> ExitCode {
	// Observe first, so that even binding to the project can be heard.
	cartridge::trace::subscribe();
	let cli = Cli::parse();
	let outcome = cli
		.check()
		.and_then(|()| project::locate(cli.dir.clone(), cli.yolo))
		.and_then(|project| {
			let runtime = tokio::runtime::Runtime::new()?;
			runtime.block_on(run(cli.command, &project))
		});
	outcome.unwrap_or_else(|error| fail(code_of(&error), error))
}

/// Every subcommand, dispatched to the function that is it.
async fn run(command: Command, project: &Project) -> Result<ExitCode> {
	match command {
		// A host in this process.
		Command::Run { key, args } => host::run(project, &key, json_arg(&args)?).await,
		Command::Launch { agent, model, args } => host::launch(project, agent, model, args).await,
		Command::Mcp => host::mcp(project).await,
		Command::Daemon => host::daemon(project).await,
		Command::Verify { cartridge } => host::verify(project, cartridge.as_deref()).await,
		// A running host, over its socket.
		Command::Send { name, data } => {
			client::send(project, json!({ "emit": name, "data": json_arg(&data)? })).await
		}
		Command::Publish { channel, data } => {
			client::send(
				project,
				json!({ "publish": channel, "data": json_arg(&data)? }),
			)
			.await
		}
		Command::Follow { channel } => client::follow(project, &channel).await,
		Command::Tail => client::tail(project).await,
		Command::Call { key, args, trace } => {
			let trace = trace.unwrap_or_else(|| cartridge::trace::mint().to_string());
			let request = json!({ "call": key, "args": json_arg(&args)?, "id": 1, "trace": trace });
			client::ask(project, request, "reply").await
		}
		Command::Reload => client::ask(project, json!({ "reload": true }), "reloaded").await,
		Command::Status => client::ask(project, json!({ "status": true }), "status").await,
		Command::Debug { state } => client::ask(project, json!({ "debug": state }), "debug").await,
		Command::Socket => {
			println!("{}", cartridge::socket::path(&project.profile).display());
			Ok(ExitCode::SUCCESS)
		}
		Command::Sweep => {
			println!("{} swept", cartridge::socket::sweep().await);
			Ok(ExitCode::SUCCESS)
		}
		// The composition, read and printed.
		Command::List => listing::list(project),
		Command::Ledger => Ok(listing::ledger(project)),
		Command::Help { what, json } => manual::help(project, &what.join(" "), json),
		Command::Settings {
			what,
			json,
			template,
		} => settings::settings(project, what.as_deref(), json, template),
		// The chain.
		Command::Up { key } => node::up(project, &key),
		Command::Enter { node, rest } => Ok(node::enter(project, &node, &rest)),
		Command::Node => node::hosted(project).await,
	}
}

/// CLI service arguments use JSON.
fn json_arg(s: &str) -> Result<Value> {
	serde_json::from_str(s).map_err(|e| Error::Argument(format!("JSON expected: {e}")))
}

/// The two ways a process is asked to stop: an interrupt from the terminal it
/// runs in, and a terminate from whatever supervises it. Either one is a
/// request, and a host that honours it gets to put its socket away.
pub(crate) async fn stopped() {
	use tokio::signal::unix::{signal, SignalKind};
	let terminate = async {
		match signal(SignalKind::terminate()) {
			Ok(mut term) => {
				term.recv().await;
			}
			Err(_) => futures::future::pending().await,
		}
	};
	tokio::select! {
		_ = tokio::signal::ctrl_c() => {}
		_ = terminate => {}
	}
}
