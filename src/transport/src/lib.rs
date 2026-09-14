//! transport — local JSON-RPC 2.0 between processes of one user.
//!
//! - [`typed`]: the adapter seam, newline-JSON framing, and owner-checked local
//!   endpoints (Unix sockets, Windows named pipes).
//! - [`rpc`]: a JSON-RPC 2.0 peer over one connection, with concurrent requests.
//! - [`cartridge`]: the cartridge side of the cartridge protocol
//!   (docs/transport.txt in the cartridge repository), for cartridges written
//!   in Rust.
//! - [`settings`]: declared settings and their defaults.
//! - [`service!`]: typed client/server pairs generated from a trait.

extern crate self as transport;

pub mod cartridge;
pub mod rpc;
pub mod settings;
pub mod typed;

pub use transport_macros::service;

/// A fresh random token: 32 bytes, lowercase hex.
pub fn token() -> String {
	let mut bytes = [0u8; 32];
	getrandom::fill(&mut bytes).expect("the operating system provides randomness");
	bytes.iter().map(|b| format!("{b:02x}")).collect()
}

// Re-exports solely for `service!`-generated code (`::transport::__private::*`).
// NOT public API — may change in any release; never import directly.
#[doc(hidden)]
pub mod __private {
	pub use bytes;
	pub use futures;
	pub use serde_json;
	pub use tokio;
	pub use tokio_util;
}
