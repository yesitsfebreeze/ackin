//! Asking the providers: the base in process, a cartridge on its event.

use serde_json::Value;

use crate::error::{Error, Result};
use crate::host::Host;
use tokio::time::Instant;

use super::own::own_answer;
use super::registry::Provider;
use super::HOST;

impl Host {
	pub(super) async fn asp_ask(
		&self,
		providers: &[&Provider],
		request: &Value,
		deadline: Option<Instant>,
	) -> Vec<Result<Value>> {
		futures::future::join_all(providers.iter().map(|p| async move {
			match p.id == HOST {
				true => Ok(own_answer(&self.participants(), &self.roster(), request)),
				false => {
					let call = self.send_to(&p.id, &p.event, request.clone());
					match deadline {
						Some(deadline) => tokio::time::timeout_at(deadline, call)
							.await
							.unwrap_or_else(|_| {
								Err(Error::Timeout("ASP provider request budget expired".into()))
							}),
						None => call.await,
					}
				}
			}
		}))
		.await
	}
}
