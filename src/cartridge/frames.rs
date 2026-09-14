//! One frame a cartridge wrote, served: registrations and events act on the
//! composition, questions are answered by id, and anything that waits is
//! answered off the read loop.

use std::sync::Arc;

use serde_json::{json, Value as Json};

use crate::error::{Error, Result};
use crate::lua::Host;
use crate::runtime::{Ctx, Error as RuntimeError, Listener, Value};

use super::link::{Link, Remote};

/// Serve one frame a cartridge wrote. Registrations and events act on the
/// composition directly; questions are answered by [`answer`].
pub(super) fn handle(
	host: &Arc<Host>,
	ctx: &Ctx,
	link: &Arc<Link>,
	name: &str,
	m: Json,
) -> Result<()> {
	if let Some(key) = m["provide"].as_str() {
		let remote = Remote {
			link: link.clone(),
			key: key.to_owned(),
		};
		return Ok(ctx.provide(key, Arc::new(remote))?);
	}
	if let Some(event) = m["on"].as_str() {
		ctx.on(event, listener(host, link, event));
		return Ok(());
	}
	if let Some(event) = m["emit"].as_str() {
		ctx.emit(event, Arc::new(m["data"].clone()));
		return Ok(());
	}
	if let Some(event) = m["send"].as_str() {
		host.send_event(event, m["data"].clone());
		return Ok(());
	}
	if let Some(channel) = m["publish"].as_str() {
		// Publishing knows nothing of its audience and asks nothing about it: the
		// stream takes the envelope, and an empty channel costs one append.
		host.runtime().stream().publish(
			channel,
			name,
			crate::stream::Kind::Data,
			m["data"].clone(),
		);
		return Ok(());
	}
	if let Some(channel) = m["subscribe"].as_str() {
		subscribe(host, link, name, channel);
		return Ok(());
	}
	if let Some(channel) = m["unsubscribe"].as_str() {
		if let Some(id) = link.sub_id(channel) {
			link.set_sub(channel, None);
			host.runtime().stream().unsubscribe(channel, id);
		}
		return Ok(());
	}
	if link.accept(&m) {
		return Ok(());
	}
	answer(host, ctx, link, name, m)
}

/// A listener the cartridge registered: every dispatch is one `event` frame
/// down the wire, and the reply is the answer.
fn listener(host: &Arc<Host>, link: &Arc<Link>, event: &str) -> Listener {
	let (host, link, event) = (host.clone(), link.clone(), event.to_owned());
	Arc::new(move |payload| {
		let (link, event, data) = (link.clone(), event.clone(), host.json_of(&payload));
		Box::pin(async move {
			link.request(json!({ "event": event, "data": data }))
				.await
				.map(|d| (!d.is_null()).then(|| Arc::new(d) as Value))
				.map_err(|e| RuntimeError::Apply(e.to_string()))
		})
	})
}

/// One subscription per channel per link: a repeat asks for what it already
/// holds, and the old pump would keep delivering underneath it.
fn subscribe(host: &Arc<Host>, link: &Arc<Link>, name: &str, channel: &str) {
	if link.sub_id(channel).is_some() {
		return;
	}
	let stream = host.runtime().stream().clone();
	let sub = stream.subscribe(channel, name, None);
	link.set_sub(channel, Some(sub.id));
	let (link, channel) = (link.clone(), channel.to_owned());
	tokio::spawn(async move {
		let mut rx = sub.rx;
		let id = sub.id;
		while let Some(envelope) = rx.recv().await {
			if !link.try_send(json!({ "channel": channel, "event": envelope })) {
				// The pipe is gone, so the leaving is announced here and the
				// forwarder is over.
				stream.unsubscribe(&channel, id);
				break;
			}
		}
	});
}

/// A question the cartridge asked, answered by id. Anything that has to wait
/// — an announce, a bridge call, a service call — is answered off the read
/// loop, so a slow answer never blocks the wire it travels back over.
fn answer(host: &Arc<Host>, ctx: &Ctx, link: &Arc<Link>, name: &str, m: Json) -> Result<()> {
	let id = m["id"].as_u64().unwrap_or(0);
	if m["bridge"] == "status" {
		link.reply(id, Ok::<_, Error>(host.bridge_status()));
		return Ok(());
	}
	if m["snapshot"] == true {
		link.reply(id, Ok::<_, Error>(host.snapshot()));
		return Ok(());
	}
	if !m["graph"].is_null() {
		// The announce is a round trip through every listening cartridge —
		// including, when the memo record asks, this one — so it answers off
		// the read loop and never blocks the wire it will travel back over.
		let (host, link, scope) = (host.clone(), link.clone(), m["graph"].clone());
		tokio::spawn(async move { link.reply(id, Ok::<_, Error>(host.graph(scope).await)) });
		return Ok(());
	}
	if m["cartridges"] == true {
		link.reply(id, Ok::<_, Error>(host.cartridges()));
		return Ok(());
	}
	if m["injections"] == true {
		link.reply(id, Ok::<_, Error>(json!(ctx.injections())));
		return Ok(());
	}
	if m["bridge"] == "call" {
		bridge_call(host, ctx, link, id, m);
		return Ok(());
	}
	if m["reload"] == true {
		host.request_reload(ctx.fiber());
		link.reply(id, Ok::<_, Error>(Json::Null));
		return Ok(());
	}
	if let Some(keys) = m["versions"].as_array() {
		let versions: Result<Vec<_>> = keys
			.iter()
			.map(|key| {
				let key = key.as_str().ok_or(Error::Invalid("service key required"))?;
				let value = ctx.get(key)?;
				Ok(value
					.downcast_ref::<crate::service::Service>()
					.map(|service| service.version()))
			})
			.collect();
		link.reply(id, versions.map(|v| json!(v)));
		return Ok(());
	}
	if let Some(key) = m["meta"].as_str() {
		link.reply(id, Ok::<_, Error>(ctx.meta(key)));
		return Ok(());
	}
	if let Some(key) = m["call"].as_str() {
		let (host, ctx, link, key, args, trace, source) = (
			host.clone(),
			ctx.clone(),
			link.clone(),
			key.to_owned(),
			m["args"].clone(),
			crate::trace::of(&m),
			name.to_owned(),
		);
		tokio::spawn(crate::trace::scope(trace, async move {
			let r = crate::observation::invoke(&host, &ctx, &source, &key, args).await;
			link.reply(id, r);
		}));
		return Ok(());
	}
	Err(Error::UnknownMessage(m))
}

/// A bridge client's call of a service its own cartridge provides, answered
/// only where the profile granted the bridge and the owner is the caller.
fn bridge_call(host: &Arc<Host>, ctx: &Ctx, link: &Arc<Link>, id: u64, call: Json) {
	if !host.bridge_enabled(ctx.fiber()) {
		link.reply(
			id,
			Err::<Json, _>(Error::Bridge("profile has not granted bridge access")),
		);
		return;
	}
	let (host, link) = (host.clone(), link.clone());
	tokio::spawn(async move {
		let result = match (
			call["owner"].as_str(),
			call["generation"].as_u64(),
			call["key"].as_str(),
		) {
			(Some(owner), Some(generation), Some(key)) => {
				host.bridge_call(owner, generation, key, call["args"].clone())
					.await
			}
			_ => Err(Error::Bridge("invalid bridge service call")),
		};
		link.reply(id, result);
	});
}
