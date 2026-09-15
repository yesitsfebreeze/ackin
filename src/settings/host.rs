use std::path::Path;
use std::sync::OnceLock;
use std::time::Duration;

use serde_json::json;

use super::{apply, defaults, get, layers, Specs};

const DOCUMENT: &str = include_str!("../../.cartridge/settings.json");

/// [`Host`] below names the same keys DOCUMENT does to give them types; a
/// mismatch is a deserialization error, not a silent divergence.
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
	pub sandbox_error_chars: usize,
	pub diagnostics_max_bytes: u64,
	pub diagnostics_queue: usize,
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

/// `deny_unknown_fields` refuses a key no declaration names; the reason is
/// gathered rather than propagated, so one typo does not end every command.
fn typed(settled: serde_json::Value, refused: &mut Vec<String>) -> Host {
	serde_json::from_value(settled).unwrap_or_else(|e| {
		refused.push(format!("host: {e}"));
		serde_json::from_value(defaults(host_specs()))
			.expect("settled host settings match their declarations")
	})
}

static HOST: OnceLock<Host> = OnceLock::new();

pub fn settle(descriptor: &Path) -> &'static Host {
	// Logged after the cell is set, never inside it: the diagnostic sink
	// itself reads `host.diagnostics_max_bytes` through here.
	let mut refused: Vec<String> = Vec::new();
	let settled = HOST.get_or_init(|| {
		let configured = layers(descriptor)
			.map(|files| get(&files, "host").cloned().unwrap_or_else(|| json!({})))
			.unwrap_or_else(|e| {
				refused.push(e.to_string());
				json!({})
			});
		let settled = apply(host_specs(), configured, "host").unwrap_or_else(|e| {
			refused.push(e.to_string());
			defaults(host_specs())
		});
		typed(settled, &mut refused)
	});
	for why in refused {
		tracing::warn!(target: "cartridge", "settings: {why}; using declared defaults");
	}
	settled
}

pub fn host() -> &'static Host {
	settle(Path::new(".cartridge"))
}

#[cfg(test)]
#[path = "../../.cartridge/tests/unit/src/settings/host.rs"]
mod tests;
