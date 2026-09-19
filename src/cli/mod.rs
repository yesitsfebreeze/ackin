mod args;
mod client;
mod host;
mod listing;
mod manual;
mod project;
mod settings;
mod scope;
mod setup;
mod trust;
mod width;

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
	let code = dispatch();
	cartridge::trace::flush();
	code
}

fn dispatch() -> ExitCode {
	// Must run before the tracing runtime: the trampoline execs and stays
	// single-threaded.
	let cli = Cli::parse();
	if let Command::Confine { policy, command } = &cli.command {
		let Err(error) = cartridge::sandbox::confine(policy, command);
		return fail(FAILED, error);
	}
	cartridge::trace::subscribe();
	if matches!(cli.command, Command::Node) {
		let runtime = match tokio::runtime::Runtime::new() {
			Ok(runtime) => runtime,
			Err(error) => return fail(FAILED, error),
		};
		return runtime
			.block_on(cartridge::node::main())
			.unwrap_or_else(|error| fail(FAILED, error));
	}
	let yolo = cli.yolo;
	let outcome = cli.check().and_then(|()| match cli.command {
		Command::Setup {
			from,
			with,
			yes,
			catalog,
		} => {
			let runtime = tokio::runtime::Runtime::new()?;
			let root = std::env::current_dir()?;
			let dir = cli.dir.map_or_else(|| root.clone(), |dir| root.join(dir));
			let ask = setup::Ask {
				from,
				with,
				yes,
				catalog,
			};
			runtime.block_on(setup::setup(&root, &dir, ask))
		}
		// Must run before locate, which reads the files this approves.
		Command::Trust {
			path,
			revoke,
			list,
			ask,
		} => trust::run(path.as_deref(), revoke, list, ask),
		command => {
			if yolo {
				std::env::set_var(cartridge::settings::YOLO_ENV, "1");
			}
			let project = project::locate(cli.dir)?;
			let runtime = tokio::runtime::Runtime::new()?;
			let outcome = runtime.block_on(run(command, &project));
			// A plain drop would block on mcp's stdin-reading blocking thread.
			runtime.shutdown_background();
			outcome
		}
	});
	outcome.unwrap_or_else(|error| fail(code_of(&error), error))
}

async fn run(command: Command, project: &Project) -> Result<ExitCode> {
	match command {
		Command::Scope { once, python } => scope::run(project, once, python.as_deref()),
		Command::Run { event, data } => host::run(project, &event, json_arg(&data)?).await,
		Command::Launch { agent, model, args } => host::launch(project, agent, model, args).await,
		Command::Mcp => host::mcp(project).await,
		Command::Daemon {
			replace,
			idle_timeout,
		} => host::daemon(project, replace, idle_timeout).await,
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
				"gather",
				json!({ "name": event, "data": json_arg(&data)? }),
			)
			.await
		}
		Command::Follow { channel, since } => client::follow(project, &channel, since).await,
		Command::Node => cartridge::node::main().await,
		Command::Confine { .. } => unreachable!("__confine runs before a project is located"),
		Command::Doctor => setup::doctor(project).await,
		Command::Setup { .. } => unreachable!("setup runs before a project is located"),
		Command::Trust { .. } => unreachable!("trust runs before a project is located"),
		Command::Status => client::ask(project, "status", Value::Null).await,
		Command::Reload { cartridge } => {
			client::ask(project, "reload", json!({ "cartridge": cartridge })).await
		}
		Command::Stop => client::ask(project, "stop", Value::Null).await,
		Command::Sweep => {
			let removed = cartridge::host::socket::sweep_all()?;
			println!("swept {removed}");
			Ok(ExitCode::SUCCESS)
		}
		Command::Socket => {
			println!(
				"{}",
				cartridge::host::socket::path(&project.descriptor)?.display()
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
