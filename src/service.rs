//! Stable service references. Calls drain before a generation is switched.
use crate::{lua::Host, reload::Reload, runtime::Value};
use parking_lot::Mutex;
use serde_json::Value as Json;
use std::sync::{
	atomic::{AtomicU64, Ordering},
	Arc,
};
static NEXT_VERSION: AtomicU64 = AtomicU64::new(1);

pub(crate) struct Service {
	value: Mutex<Value>,
	version: AtomicU64,
	pub(crate) reload: Reload,
}

impl Service {
	pub(crate) fn new(value: Value, reload: Reload) -> Self {
		Self {
			value: Mutex::new(value),
			version: AtomicU64::new(NEXT_VERSION.fetch_add(1, Ordering::Relaxed)),
			reload,
		}
	}
	pub(crate) fn value(&self) -> Value {
		self.value.lock().clone()
	}
	pub(crate) fn version(&self) -> u64 {
		let _value = self.value.lock();
		self.version.load(Ordering::Acquire)
	}
	pub(crate) fn switch(&self, value: Value) -> Value {
		let mut current = self.value.lock();
		let old = std::mem::replace(&mut *current, value);
		self.version.store(
			NEXT_VERSION.fetch_add(1, Ordering::Relaxed),
			Ordering::Release,
		);
		old
	}

	pub(crate) async fn call(&self, host: &Host, mut args: Json) -> Result<Json, String> {
		loop {
			let mut epoch = self.reload.epoch();
			let guard = self.reload.gate().read_owned().await;
			let previous = *epoch.borrow();
			crate::observation::dispatch(self);
			let result = host.invoke_raw(self.value(), args).await;
			drop(guard);
			let result = result?;
			let Some(resume) = result.get("$cartridge_resume") else {
				return Ok(result);
			};
			args = resume.clone();
			crate::observation::resuming(self);
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

#[cfg(test)]
#[path = "../.cartridge/tests/unit/service/version_tests.rs"]
mod version_tests;
