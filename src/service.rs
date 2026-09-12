//! Stable service references. Calls drain before a generation is switched.
use crate::{lua::Host, reload::Reload, runtime::Value};
use parking_lot::Mutex;
use serde_json::Value as Json;
use std::sync::Arc;

pub(crate) struct Service {
	value: Mutex<Value>,
	pub(crate) reload: Reload,
}

impl Service {
	pub(crate) fn new(value: Value, reload: Reload) -> Self {
		Self {
			value: Mutex::new(value),
			reload,
		}
	}
	pub(crate) fn value(&self) -> Value {
		self.value.lock().clone()
	}
	pub(crate) fn switch(&self, value: Value) -> Value {
		std::mem::replace(&mut *self.value.lock(), value)
	}
	pub(crate) async fn call(&self, host: &Host, mut args: Json) -> Result<Json, String> {
		loop {
			let mut epoch = self.reload.epoch();
			let guard = self.reload.gate().read_owned().await;
			let previous = *epoch.borrow();
			let result = host.invoke_raw(self.value(), args).await;
			drop(guard);
			let result = result?;
			let Some(resume) = result.get("$cartridge_resume") else {
				return Ok(result);
			};
			args = resume.clone();
			epoch
				.wait_for(|epoch| *epoch != previous)
				.await
				.map_err(|_| "cartridge removed".to_owned())?;
		}
	}

	pub(crate) async fn prepare(&self) -> Result<(), String> {
		let value = self.value();
		if let Some(remote) = value.downcast_ref::<crate::cartridge::Remote>() {
			remote.prepare_reload().await?;
		}
		Ok(())
	}
	pub(crate) fn process_id(&self) -> Option<usize> {
		self.value()
			.downcast_ref::<crate::cartridge::Remote>()
			.map(|remote| remote.process_id())
	}
	pub(crate) async fn cancel(&self) {
		if let Some(remote) = self.value().downcast_ref::<crate::cartridge::Remote>() {
			remote.cancel_reload().await;
		}
	}
}

pub(crate) fn shared(value: Value, reload: Reload) -> Value {
	Arc::new(Service::new(value, reload))
}
