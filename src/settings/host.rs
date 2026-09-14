//! The host's own settings: declared in `.cartridge/settings.json`,
//! settled once against the profile's files, read everywhere as one answer.

use std::path::Path;
use std::sync::OnceLock;
use std::time::Duration;

use serde_json::json;

use super::{apply, defaults, get, layers, Specs};
use crate::error::Error;

/// The host's own document. It has no `cartridge.json` — nothing composes the
/// host — so its declarations live in `.cartridge/settings.json`, in the same
/// shape a cartridge's `settings` block uses. Embedded rather than read from
/// disk: these are the values the binary was built with, and a host that could
/// not find its own declarations would have no defaults to fall back to.
const DOCUMENT: &str = include_str!("../../.cartridge/settings.json");

/// What the host declares, parsed once off [`DOCUMENT`]. The numbers, bounds
/// and documentation live there and nowhere else; [`Host`] below names the same
/// keys to give them types, and a mismatch between the two is a deserialization
/// error rather than a silent divergence.
pub fn host_specs() -> &'static Specs {
	static SPECS: OnceLock<Specs> = OnceLock::new();
	SPECS.get_or_init(|| {
		#[derive(serde::Deserialize)]
		struct Document {
			settings: Specs,
		}
		let document: Document =
			serde_json::from_str(DOCUMENT).expect("the host's own settings.json");
		document.settings
	})
}

/// The host's settled settings: every key of [`DOCUMENT`], typed. Read it; do
/// not re-derive it. Durations are kept as the numbers the document declares
/// and handed out as `Duration` by the methods below, so the unit is in the
/// key's name at every layer a person reads.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Host {
	pub lifecycle_queue: usize,
	pub startup_timeout_secs: u64,
	pub event_timeout_ms: u64,
	pub shutdown_timeout_secs: u64,
	pub verify_timeout_secs: u64,
	pub watch_debounce_ms: u64,
	pub mcp_reply_queue: usize,
	pub proxy_key_bytes: usize,
	pub sandbox_error_chars: usize,
	pub diagnostics_max_bytes: u64,
	pub lua_memory_bytes: usize,
	pub lua_instruction_budget: u64,
}

impl Host {
	pub fn startup_timeout(&self) -> Duration {
		Duration::from_secs(self.startup_timeout_secs)
	}

	pub fn shutdown_timeout(&self) -> Duration {
		Duration::from_secs(self.shutdown_timeout_secs)
	}

	pub fn verify_timeout(&self) -> Duration {
		Duration::from_secs(self.verify_timeout_secs)
	}

	pub fn watch_debounce(&self) -> Duration {
		Duration::from_millis(self.watch_debounce_ms)
	}
}

/// The settled table as [`Host`]. A key no declaration names survives the
/// merge and `deny_unknown_fields` refuses it; one typo must not end every
/// command, so the reason is said and the declared defaults stand.
fn typed(settled: serde_json::Value) -> Host {
	serde_json::from_value(settled).unwrap_or_else(|e| {
		tracing::warn!(target: "cartridge", "settings: host: {e}; using declared defaults");
		serde_json::from_value(defaults(host_specs()))
			.expect("settled host settings match their declarations")
	})
}

static HOST: OnceLock<Host> = OnceLock::new();

/// The `host` table of the configuration files, settled against [`DOCUMENT`].
///
/// A file that will not read, or a value out of its declared range, does not
/// take the host down: the reason goes to stderr once and the declared defaults
/// stand. A limit is the thing that keeps a runaway bounded, and refusing to
/// start because someone typed a limit wrongly trades a bounded system for no
/// system at all.
///
/// `profile` is where the project's configuration file sits. The CLI settles
/// against the profile it resolved, before anything else runs; everything
/// downstream reads that one settled answer through [`host`].
pub fn settle(profile: &Path) -> &'static Host {
	HOST.get_or_init(|| {
		let say = |e: Error| tracing::warn!(target: "cartridge", "settings: {e}; using declared defaults");
		let configured = layers(profile)
			.map(|files| get(&files, "host").cloned().unwrap_or_else(|| json!({})))
			.unwrap_or_else(|e| {
				say(e);
				json!({})
			});
		let settled = apply(host_specs(), configured, "host").unwrap_or_else(|e| {
			say(e);
			defaults(host_specs())
		});
		typed(settled)
	})
}

/// The host's settings, settling them against the profile beside the working
/// directory on first use — which is what a process with no CLI to settle
/// for it, gets.
pub fn host() -> &'static Host {
	settle(Path::new(".cartridge"))
}

#[cfg(test)]
#[path = "../../.cartridge/tests/unit/src/settings/host.rs"]
mod tests;
