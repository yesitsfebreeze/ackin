mod files;
mod host;

use serde_json::Value as Json;

use crate::error::{Error, Result};

pub use crate::transport::settings::{
	declared, defaults, get, merge, set, undeclared, yolo, Kind, Spec, Specs, YOLO_ENV,
};
pub use files::{global_path, layers, project_path, read, Sources};
pub use host::{host, host_specs, settle, Host};

pub fn apply(specs: &Specs, mut config: Json, at: &str) -> Result<Json> {
	if yolo() && specs.contains_key("yolo") {
		merge(&mut config, serde_json::json!({"yolo": true}));
	}
	crate::transport::settings::apply(specs, config, at).map_err(Error::Settings)
}
