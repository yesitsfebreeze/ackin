//! A Lua cartridge: served by the host in-process on its own socket, on the
//! same terms as a process cartridge.

use std::sync::Arc;

use futures::FutureExt;
use mlua::{Function, Lua, LuaSerdeExt, MetaMethod, UserData, UserDataMethods};
use serde_json::{json, Value};
use transport::cartridge::{Ctx, Directory};

use crate::error::{Error, Result};

use super::{Host, Plan, Running};

pub(super) async fn start(
	host: &Arc<Host>,
	plan: &Plan,
	apply: Function,
	directory: &Directory,
) -> Result<Running> {
	let settings = crate::settings::host();
	let socket = host.socket(&plan.id);
	let _ = std::fs::remove_file(&socket);
	let listener = transport::cartridge::listen(&socket)
		.await
		.map_err(|e| Error::process(&plan.id, e))?;
	let ctx = Ctx::new(host.host_token(), settings.startup_timeout());
	let lua = host.lua.clone();
	let on = plan.on.clone();
	let task = tokio::spawn(transport::cartridge::serve(
		listener,
		ctx.clone(),
		Box::new(move |ctx, config| {
			async move {
				let config = lua.to_value(&config).map_err(|e| e.to_string())?;
				let returned: mlua::Value = apply
					.call_async((
						LuaCtx {
							ctx: ctx.clone(),
							lua: lua.clone(),
							on,
						},
						config,
					))
					.await
					.map_err(|e| e.to_string())?;
				if let mlua::Value::Function(dispose) = returned {
					ctx.on_dispose(move || async move {
						if let Err(error) = dispose.call_async::<()>(()).await {
							tracing::warn!(target: "cartridge", "disposer failed: {error}");
						}
					});
				}
				Ok(())
			}
			.boxed()
		}),
	));
	let (peer, _incoming) = super::connect(&socket, host.host_token()).await?;
	let apply = peer.call(
		"apply",
		json!({ "name": plan.name, "config": plan.config, "directory": directory }),
	);
	match tokio::time::timeout(settings.startup_timeout(), apply).await {
		Ok(Ok(_)) => {}
		Ok(Err(error)) => {
			ctx.stop();
			return Err(Error::process(&plan.id, error.message));
		}
		Err(_) => {
			ctx.stop();
			return Err(Error::Timeout(plan.id.clone()));
		}
	}
	Ok(Running {
		peer,
		stop: Box::new(move |peer| {
			Box::pin(async move {
				let shutdown = crate::settings::host().shutdown_timeout();
				let _ = tokio::time::timeout(shutdown, peer.call("dispose", json!({}))).await;
				ctx.stop();
				let _ = tokio::time::timeout(shutdown, task).await;
				let _ = std::fs::remove_file(&socket);
			})
		}),
	})
}

#[derive(Clone)]
struct LuaCtx {
	ctx: Ctx,
	lua: Lua,
	on: Vec<String>,
}

fn json(lua: &Lua, value: mlua::Value) -> mlua::Result<Value> {
	lua.from_value(value)
}

fn external(error: String) -> mlua::Error {
	mlua::Error::RuntimeError(error)
}

impl LuaCtx {
	fn handler(
		&self,
		f: Function,
	) -> impl Fn(Value) -> futures::future::BoxFuture<'static, transport::cartridge::Result<Value>>
	       + Send
	       + Sync
	       + 'static {
		let lua = self.lua.clone();
		move |args| {
			let (lua, f) = (lua.clone(), f.clone());
			async move {
				let args = lua.to_value(&args).map_err(|e| e.to_string())?;
				let out: mlua::Value = f.call_async(args).await.map_err(|e| e.to_string())?;
				lua.from_value(out).map_err(|e| e.to_string())
			}
			.boxed()
		}
	}

	fn caller(&self, key: String) -> mlua::Result<Function> {
		let ctx = self.ctx.clone();
		self.lua
			.create_async_function(move |lua, args: mlua::Value| {
				let (ctx, key) = (ctx.clone(), key.clone());
				async move {
					let out = ctx.call(&key, json(&lua, args)?).await.map_err(external)?;
					lua.to_value(&out)
				}
			})
	}
}

impl UserData for LuaCtx {
	fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
		methods.add_method("name", |_, this, ()| Ok(this.ctx.name()));
		methods.add_method("needs", |_, this, ()| Ok(this.ctx.needs()));
		methods.add_method("provide", |_, this, (key, f): (String, Function)| {
			this.ctx.provide(&key, this.handler(f));
			Ok(())
		});
		methods.add_method("on", |_, this, (name, f): (String, Function)| {
			if !this.on.contains(&name) {
				return Err(external(format!("`{name}` is not declared in `on`")));
			}
			this.ctx.on(&name, this.handler(f));
			Ok(())
		});
		methods.add_method("on_dispose", |_, this, f: Function| {
			this.ctx.on_dispose(move || async move {
				if let Err(error) = f.call_async::<()>(()).await {
					tracing::warn!(target: "cartridge", "disposer failed: {error}");
				}
			});
			Ok(())
		});
		methods.add_async_method(
			"call",
			|lua, this, (key, args): (String, mlua::Value)| async move {
				let out = this
					.ctx
					.call(&key, json(&lua, args)?)
					.await
					.map_err(external)?;
				lua.to_value(&out)
			},
		);
		methods.add_method("emit", |lua, this, (name, data): (String, mlua::Value)| {
			this.ctx.emit(&name, json(lua, data)?);
			Ok(())
		});
		methods.add_method(
			"notify",
			|lua, this, (name, data): (String, mlua::Value)| {
				this.ctx.notify(&name, json(lua, data)?);
				Ok(())
			},
		);
		methods.add_async_method(
			"bail",
			|lua, this, (name, data): (String, mlua::Value)| async move {
				let answer = this
					.ctx
					.bail(&name, json(&lua, data)?)
					.await
					.map_err(external)?;
				match answer {
					Some(answer) => lua.to_value(&answer),
					None => Ok(mlua::Value::Nil),
				}
			},
		);
		methods.add_async_method(
			"parallel",
			|lua, this, (name, data): (String, mlua::Value)| async move {
				this.ctx
					.parallel(&name, json(&lua, data)?)
					.await
					.map_err(external)
			},
		);
		methods.add_async_method(
			"gather",
			|lua, this, (name, data): (String, mlua::Value)| async move {
				let rows = lua.create_table()?;
				for (from, data) in this.ctx.gather(&name, json(&lua, data)?).await {
					let row = lua.create_table()?;
					row.set("from", from)?;
					row.set("data", lua.to_value(&data)?)?;
					rows.push(row)?;
				}
				Ok(rows)
			},
		);
		methods.add_method(
			"publish",
			|lua, this, (channel, data): (String, mlua::Value)| {
				this.ctx.publish(&channel, json(lua, data)?);
				Ok(())
			},
		);
		methods.add_async_method(
			"subscribe",
			|_, this, (cartridge, channel, f): (String, String, Function)| async move {
				let lua = this.lua.clone();
				this.ctx
					.subscribe(&cartridge, &channel, None, move |envelope| {
						let (lua, f) = (lua.clone(), f.clone());
						async move {
							if let Ok(envelope) = lua.to_value(&envelope) {
								if let Err(error) = f.call_async::<()>(envelope).await {
									tracing::warn!(target: "cartridge", "subscriber failed: {error}");
								}
							}
						}
					})
					.await
					.map_err(external)
			},
		);
		methods.add_meta_method(MetaMethod::Index, |_, this, key: String| {
			if this.ctx.needs().contains(&key) {
				return this.caller(key).map(mlua::Value::Function);
			}
			Err(external(format!("`{key}` is not a need of this cartridge")))
		});
	}
}
