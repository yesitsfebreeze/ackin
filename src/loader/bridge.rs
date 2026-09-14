//! The bridge: a cartridge's client module calling services its own
//! cartridge provides, where the profile granted it.

use std::path::Path;

use crate::error::{Error, Result};
use crate::lua::Host;

impl Host {
	/// A profile explicitly grants the bridge client access to active plugins'
	/// own services. This avoids making every optional panel a required
	/// dependency.
	pub(crate) fn bridge_enabled(&self, uid: crate::runtime::Uid) -> bool {
		self.loaded.lock().iter().any(|slot| {
			slot.entry.config["bridge"] == true
				&& slot.fiber.as_ref().is_some_and(|fiber| fiber.uid() == uid)
		})
	}

	pub async fn bridge_call(
		&self,
		owner: &str,
		generation: u64,
		key: &str,
		args: serde_json::Value,
	) -> Result<serde_json::Value> {
		let value = {
			let loaded = self.loaded.lock();
			let slot = loaded
				.iter()
				.find(|slot| slot.entry.id == owner)
				.ok_or(Error::Bridge("bridge owner unloaded"))?;
			let fiber = slot
				.fiber
				.as_ref()
				.ok_or(Error::Bridge("bridge owner inactive"))?;
			if fiber.uid() != generation || fiber.state() != Some(crate::runtime::State::Active) {
				return Err(Error::Bridge("bridge generation is no longer active"));
			}
			if !slot.sources.iter().any(|s| {
				s.path
					.extension()
					.is_some_and(|e| e == "tsx" || e == "ts" || e == "js" || e == "jsx")
			}) {
				return Err(Error::Bridge("cartridge has no client module"));
			}
			let reg = self.rt.reg.lock();
			reg.owns(generation, key)
				.and_then(|realm| reg.value(realm))
				.ok_or(Error::Bridge(
					"a bridge client may only call services provided by its own cartridge",
				))?
		};
		self.invoke(value, args).await
	}

	/// Client descriptors belong to committed, active generations. Failed
	/// candidates cannot change the status, and plugins without a client module
	/// have no descriptor.
	pub fn bridge_status(&self) -> serde_json::Value {
		let loaded = self.loaded.lock();
		serde_json::Value::Array(
			loaded
				.iter()
				.filter_map(|slot| {
					let fiber = slot.fiber.as_ref()?;
					if fiber.state() != Some(crate::runtime::State::Active) {
						return None;
					}
					let module = slot.sources.iter().find(|source| {
						source
							.path
							.extension()
							.is_some_and(|e| e == "tsx" || e == "ts" || e == "js" || e == "jsx")
					})?;
					let services: Vec<_> = self.rt.reg.lock().provided(fiber.uid()).into_iter().map(|(key, _)| key).collect();
					Some(
						serde_json::json!({"id": slot.entry.id, "generation": fiber.uid(), "module": module.path, "services": services}),
					)
				})
				.collect(),
		)
	}

	/// Folders of enabled cartridges, for records they ship (`.cartridge/memos`).
	/// Disabled entries have no fiber and are absent.
	pub fn cartridges(&self) -> serde_json::Value {
		let loaded = self.loaded.lock();
		serde_json::Value::Array(
			loaded
				.iter()
				.filter(|slot| slot.fiber.is_some())
				.map(|slot| {
					let dir = slot.entry.file(&self.dir).parent().map(Path::to_path_buf);
					serde_json::json!({"id": slot.entry.id, "dir": dir})
				})
				.collect(),
		)
	}
}
