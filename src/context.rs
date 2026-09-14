use crate::lua::{block_on, Host};
use crate::runtime::{Ctx, FiberHandle, Listener};
use mlua::{Function, LuaSerdeExt, MetaMethod, UserData, UserDataMethods};
use std::sync::Arc;

pub(crate) struct LuaCtx {
	host: Arc<Host>,
	ctx: Ctx,
	cartridge: String,
}

impl LuaCtx {
	pub(crate) fn new(host: Arc<Host>, ctx: Ctx, cartridge: String) -> Self {
		Self {
			host,
			ctx,
			cartridge,
		}
	}

	fn get(&self, key: &str) -> mlua::Result<mlua::Value> {
		match self.ctx.get(key) {
			Ok(v) => Ok(self.host.to_lua(&v)),
			// A need the chain already resolved has no provider in this host's
			// store: it lives in the dependency that launched this node. The
			// chain link is the binding, so the get becomes a remote — the
			// author calls it like any other provided key, and the frames
			// cross the socket. An unbound need has no remote and keeps the
			// store's own refusal.
			Err(_) => match self.host.dependency(key) {
				Some(remote) => Ok(self.host.to_lua(&remote)),
				None => self
					.ctx
					.get(key)
					.map(|v| self.host.to_lua(&v))
					.map_err(external),
			},
		}
	}

	fn derive(&self, ctx: Ctx) -> Self {
		Self {
			host: self.host.clone(),
			ctx,
			cartridge: self.cartridge.clone(),
		}
	}
}

pub(crate) struct LuaFiber(Arc<Host>, FiberHandle, crate::reload::Reload);

fn external(e: crate::runtime::Error) -> mlua::Error {
	mlua::Error::external(e)
}

impl UserData for LuaFiber {
	fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
		methods.add_method("uid", |_, this, ()| Ok(this.1.uid()));
		methods.add_method("state", |_, this, ()| {
			Ok(this.1.state().map(|s| format!("{s:?}")))
		});
		methods.add_method("error", |_, this, ()| Ok(this.1.error()));
		methods.add_method("dispose", |_, this, ()| {
			block_on(this.1.dispose());
			Ok(())
		});
		methods.add_method("wait", |_, this, ()| {
			block_on(this.1.settled());
			Ok(())
		});
		methods.add_method("reload", |_, this, ()| {
			// The handle names the generation it was minted for, but the ask
			// addresses the node: what persists across a swap is the shared
			// reload transaction, so the live generation is found through it.
			this.0.request_reload_for(this.1.uid(), this.2.clone());
			Ok(())
		});
	}
}

impl UserData for LuaCtx {
	fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
		methods.add_method("effect", |_, this, f: Function| {
			let disposer: mlua::Value = f.call(())?;
			let host = this.host.clone();
			this.ctx.effect_sync(move || host.disposer(disposer));
			Ok(())
		});
		methods.add_method("provide", |_, this, (key, value): (String, mlua::Value)| {
			this.ctx.provide(&key, Arc::new(value)).map_err(external)
		});
		methods.add_method("get", |_, this, key: String| this.get(&key));
		methods.add_method("peek", |_, this, key: String| {
			Ok(this.ctx.peek(&key).map(|v| this.host.to_lua(&v)))
		});
		methods.add_method("meta", |lua, this, key: String| {
			lua.to_value(&this.ctx.meta(&key))
		});
		methods.add_method("isolate", |_, this, key: String| {
			Ok(this.derive(this.ctx.isolate(&key)))
		});
		methods.add_method(
			"intercept",
			|_, this, (key, meta): (String, mlua::Value)| {
				let meta = this.host.to_json(meta);
				Ok(this.derive(this.ctx.intercept(&key, meta)))
			},
		);
		methods.add_method("on", |_, this, (name, f): (String, Function)| {
			let host = this.host.clone();
			let listener: Listener = Arc::new(move |payload| {
				let result = f.call::<mlua::Value>(host.to_lua(&payload));
				Box::pin(async move {
					match result {
						Ok(mlua::Value::Nil) => Ok(None),
						Ok(v) => Ok(Some(Arc::new(v) as crate::runtime::Value)),
						Err(e) => Err(crate::runtime::Error::Apply(e.to_string())),
					}
				})
			});
			this.ctx.on(&name, listener);
			Ok(())
		});
		methods.add_method("emit", |_, this, (name, payload): (String, mlua::Value)| {
			this.ctx.emit(&name, Arc::new(payload));
			Ok(())
		});
		methods.add_method("bail", |_, this, (name, payload): (String, mlua::Value)| {
			let answer = block_on(this.ctx.bail(&name, Arc::new(payload))).map_err(external)?;
			Ok(answer.map(|v| this.host.to_lua(&v)))
		});
		methods.add_method(
			"parallel",
			|_, this, (name, payload): (String, mlua::Value)| {
				block_on(this.ctx.parallel(&name, Arc::new(payload))).map_err(external)
			},
		);
		// One row per answering cartridge: `{from = <entry id>, data = <answer>}`.
		// The name is stamped here from the registry rather than taken from the
		// answer, so a contribution cannot claim to come from somewhere else.
		methods.add_method(
			"gather",
			|lua, this, (name, payload): (String, mlua::Value)| {
				let answers = block_on(this.ctx.gather(&name, Arc::new(payload)));
				let labels = this.host.labels();
				let rows = lua.create_table()?;
				for (uid, answer) in answers {
					let row = lua.create_table()?;
					row.set("from", labels.get(&uid).cloned())?;
					row.set("data", this.host.to_lua(&answer))?;
					rows.push(row)?;
				}
				Ok(rows)
			},
		);
		methods.add_method("send", |_, this, (name, data): (String, mlua::Value)| {
			let data = this.host.to_json(data);
			this.host.send_event(&name, data);
			Ok(())
		});
		methods.add_method(
			"publish",
			|_, this, (channel, payload): (String, mlua::Value)| {
				let data = this.host.to_json(payload);
				this.host.runtime().stream().publish(
					&channel,
					&this.cartridge,
					crate::stream::Kind::Data,
					data,
				);
				Ok(())
			},
		);
		methods.add_method("subscribe", |_, this, (channel, f): (String, Function)| {
			let stream = this.host.runtime().stream().clone();
			let sub = stream.subscribe(&channel, &this.cartridge, None);
			// One pump per subscription, handling in arrival order so two events
			// published one after the other never race inside the Lua function.
			let pump_host = this.host.clone();
			let pump_stream = stream.clone();
			let pump_channel = channel.clone();
			let pump = tokio::spawn(async move {
				let mut rx = sub.rx;
				while let Some(envelope) = rx.recv().await {
					let value = match pump_host.lua.to_value(&envelope) {
						Ok(value) => value,
						Err(_) => break,
					};
					if f.call::<mlua::Value>(value).is_err() {
						// A function that dies mid-watch leaves announced, not silent.
						pump_stream.unsubscribe(&pump_channel, sub.id);
						break;
					}
				}
			});
			this.ctx.effect_sync(move || {
				Box::new(move || {
					pump.abort();
					stream.unsubscribe(&channel, sub.id);
					Box::pin(async {})
				})
			});
			Ok(())
		});
		methods.add_method(
			"cartridge",
			|_, this, (path, config): (String, mlua::Value)| {
				let config = this.host.to_json(config);
				let file = this.host.dir().join(&path);
				let (mut component, _sources) =
					this.host.load_component(&file, config.clone(), &[])?;
				// A composed node is a long-lived participant of the tree, like a
				// profile entry: its services are stable across a swap and it
				// carries the rebuild that re-reads its source for the next one.
				component.resident = true;
				let rebuild_host = this.host.clone();
				let rebuild_path = file.clone();
				component.rebuild = Some(Arc::new(move |_ctx| {
					let (host, path, config) =
						(rebuild_host.clone(), rebuild_path.clone(), config.clone());
					host.load_component(&path, config, &[])
						.map(|(component, _)| component)
						.map_err(|e| {
							crate::runtime::Error::Apply(format!("rebuild of {path:?}: {e}"))
						})
				}));
				// The handle keeps the node's transaction, so a reload fired
				// after a swap re-finds the live generation through it.
				let reload = component.reload.clone();
				Ok(LuaFiber(
					this.host.clone(),
					this.ctx.cartridge(component),
					reload,
				))
			},
		);
		methods.add_method("name", |_, this, ()| Ok(this.cartridge.clone()));
		methods.add_method("state", |_, this, ()| {
			let state = this.ctx.runtime().state_of(this.ctx.fiber());
			Ok(state.map(|s| format!("{s:?}")))
		});
		methods.add_meta_method(MetaMethod::Index, |_, this, key: String| this.get(&key));
	}
}
