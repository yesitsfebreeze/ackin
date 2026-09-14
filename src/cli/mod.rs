//! The command line: parse, bind to the project, run one subcommand.

mod args;
mod client;
mod host;
mod listing;
mod manual;
mod project;
mod settings;

use std::process::ExitCode;

use cartridge::{Error, Result};
use clap::Parser;
use serde_json::{json, Value};

use args::{Cli, Command};
pub(crate) use project::Project;

pub(crate) const FAILED: u8 = 1;
pub(crate) const USAGE: u8 = 2;

pub(crate) fn fail(code: u8, why: impl std::fmt::Display) -> ExitCode {
	eprintln!("{why}");
	ExitCode::from(code)
}

fn code_of(error: &Error) -> u8 {
	match error {
		Error::Argument(_) => USAGE,
		_ => FAILED,
	}
}

pub fn main() -> ExitCode {
	cartridge::trace::subscribe();
	let cli = Cli::parse();
	if matches!(cli.command, Command::Node) {
		let runtime = match tokio::runtime::Runtime::new() {
			Ok(runtime) => runtime,
			Err(error) => return fail(FAILED, error),
		};
		return runtime
			.block_on(cartridge::node::main())
			.unwrap_or_else(|error| fail(FAILED, error));
	}
	let outcome = cli
		.check()
		.and_then(|()| project::locate(cli.dir.clone(), cli.yolo))
		.and_then(|project| {
			let runtime = tokio::runtime::Runtime::new()?;
			runtime.block_on(run(cli.command, &project))
		});
	outcome.unwrap_or_else(|error| fail(code_of(&error), error))
}

async fn run(command: Command, project: &Project) -> Result<ExitCode> {
	match command {
		Command::Run { event, data } => host::run(project, &event, json_arg(&data)?).await,
		Command::Launch { agent, model, args } => host::launch(project, agent, model, args).await,
		Command::Mcp => host::mcp(project).await,
		Command::Daemon => host::daemon(project).await,
		Command::Verify { cartridge } => host::verify(project, cartridge.as_deref()).await,
		Command::Call { event, data } => {
			client::ask(
				project,
				"bail",
				json!({ "name": event, "data": json_arg(&data)? }),
			)
			.await
		}
		Command::Send { event, data } => {
			client::ask(
				project,
				"emit",
				json!({ "name": event, "data": json_arg(&data)? }),
			)
			.await
		}
		Command::Follow { channel, since } => client::follow(project, &channel, since).await,
		Command::Node => cartridge::node::main().await,
		Command::Status => client::ask(project, "status", Value::Null).await,
		Command::Reload { cartridge } => {
			client::ask(project, "reload", json!({ "cartridge": cartridge })).await
		}
		Command::Stop => client::ask(project, "stop", Value::Null).await,
		Command::Socket => {
			println!(
				"{}",
				cartridge::host::socket::path(&project.profile)?.display()
			);
			Ok(ExitCode::SUCCESS)
		}
		Command::List => listing::list(project),
		Command::Ledger => Ok(listing::ledger(project)),
		Command::Help { what, json } => manual::help(project, &what.join(" "), json),
		Command::Settings {
			what,
			json,
			template,
		} => settings::settings(project, what.as_deref(), json, template),
	}
}

fn json_arg(s: &str) -> Result<Value> {
	serde_json::from_str(s).map_err(|e| Error::Argument(format!("JSON expected: {e}")))
}

/// An interrupt from the terminal or a terminate from a supervisor.
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
