//! Settings: every tunable value is declared, then settled in layers:
//! the declaration, the document's `config`, `~/.cartridge/config.lua`, the
//! project's `.cartridge/config.lua`, and the profile entry's `config`.
//! The declaration format lives in [`crate::transport::settings`]; the host's own keys
//! are declared in `.cartridge/settings.json` and read through [`host`].

mod files;
mod host;

use serde_json::Value as Json;

use crate::error::{Error, Result};

pub use crate::transport::settings::{
	declared, defaults, get, merge, set, undeclared, Kind, Spec, Specs,
};
pub use files::{global_path, layers, project_path, read, source};
pub use host::{host, host_specs, settle, Host};

/// Fill `config` from the declarations and refuse what it names wrongly.
pub fn apply(specs: &Specs, config: Json, at: &str) -> Result<Json> {
	crate::transport::settings::apply(specs, config, at).map_err(Error::Settings)
}
