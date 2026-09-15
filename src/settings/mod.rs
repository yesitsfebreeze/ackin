//! Settings: every tunable value is declared, then settled in layers:
//! the declaration, the document's `config`, `~/.cartridge/config.lua`, the
//! project's `.cartridge/config.lua`, and the descriptor entry's `config`.
//! The declaration format lives in [`crate::transport::settings`]; the host's own keys
//! are declared in `.cartridge/settings.json` and read through [`host`].

mod files;
mod host;

use serde_json::Value as Json;

use crate::error::{Error, Result};

pub use crate::transport::settings::{
	declared, defaults, get, merge, set, undeclared, yolo, Kind, Spec, Specs, YOLO_ENV,
};
pub use files::{global_path, layers, project_path, read, Sources};
pub use host::{host, host_specs, settle, Host};

/// Fill `config` from the declarations and refuse what it names wrongly.
///
/// `--yolo` lands here, over every file layer and only where the cartridge
/// declares the key: automatic execution is the command's word about this run,
/// so no configuration file names which cartridges it reaches.
pub fn apply(specs: &Specs, mut config: Json, at: &str) -> Result<Json> {
	if yolo() && specs.contains_key("yolo") {
		merge(&mut config, serde_json::json!({"yolo": true}));
	}
	crate::transport::settings::apply(specs, config, at).map_err(Error::Settings)
}
