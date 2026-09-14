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
		Command::Run { key, args } => host::run(project, &key, json_arg(&args)?).await,
		Command::Launch { agent, model, args } => host::launch(project, agent, model, args).await,
		Command::Mcp => host::mcp(project).await,
		Command::Daemon => host::daemon(project).await,
		Command::Verify { cartridge } => host::verify(project, cartridge.as_deref()).await,
		Command::Call { key, args, trace } => {
			let trace = trace.unwrap_or_else(|| cartridge::trace::mint().to_string());
			client::ask(
				project,
				"call",
				json!({ "key": key, "args": json_arg(&args)?, "trace": trace }),
			)
			.await
		}
		Command::Send { name, data } => {
			client::ask(
				project,
				"emit",
				json!({ "name": name, "data": json_arg(&data)? }),
			)
			.await
		}
		Command::Follow { channel, since } => client::follow(project, &channel, since).await,
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
