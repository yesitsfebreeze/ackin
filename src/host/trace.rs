//! Host lifecycle and diagnostics feed the same declared trace writer as nodes.
use super::Host;
use crate::trace::activity::Recorder;
use std::sync::Arc;

impl Host {
	pub(super) fn trace_recorder(self: &Arc<Self>) -> &Recorder {
		self.trace_queue.get_or_init(|| {
			let weak = Arc::downgrade(self);
			crate::trace::activity::delivery(move |activity| {
				let host = weak.upgrade();
				async move {
					let Some(host) = host else {
						return Ok(());
					};
					if !host
						.participants()
						.iter()
						.any(|p| p.events.contains_key("trace"))
					{
						return Ok(());
					}
					host.bail("trace", crate::trace::activity::append(activity))
						.await
						.map_err(|e| e.to_string())?
						.ok_or_else(|| "trace has no active writer".to_owned())
						.map(|_| ())
				}
			})
		})
	}
}
