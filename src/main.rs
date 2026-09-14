//! The `cartridge` command: cartridges on a socket, run in the back, handle
//! the events. Every subcommand lives under [`cli`]; this is the entry point.

mod cli;

fn main() -> std::process::ExitCode {
	cli::main()
}
