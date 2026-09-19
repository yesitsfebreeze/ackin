//! Asking the providers: the base in process, a cartridge on its event.

use serde_json::Value;

use crate::error::Result;
use crate::host::Host;

use super::own::own_answer;
use super::registry::Provider;
use super::HOST;

impl Host {
	pub(super) async fn asp_ask(
		&self,
		providers: &[&Provider],
		request: &Value,
	) -> Vec<Result<Value>> {
		futures::future::join_all(providers.iter().map(|p| async move {
			match p.id == HOST {
				true => Ok(own_answer(&self.participants(), request)),
				false => self.send_to(&p.id, &p.event, request.clone()).await,
			}
		}))
		.await
	}
}
