//! The wire between the base and its nodes (docs/transport.txt): local
//! JSON-RPC 2.0 between processes of one user. Internal to the base; a
//! cartridge never speaks it, its `init.lua` uses the injected `cartridge`.
//!
//! - [`typed`]: framing and owner-checked local endpoints.
//! - [`rpc`]: a JSON-RPC 2.0 peer over one connection.
//! - [`cartridge`]: the node side: the directory, events, streams, serving.
//! - [`settings`]: declared settings and their defaults.

pub mod cartridge;
pub mod rpc;
pub mod settings;
pub mod typed;

/// A fresh random token: 32 bytes, lowercase hex.
pub fn token() -> String {
	let mut bytes = [0u8; 32];
	getrandom::fill(&mut bytes).expect("the operating system provides randomness");
	bytes.iter().map(|b| format!("{b:02x}")).collect()
}
