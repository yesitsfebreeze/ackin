// The cartridge side of the cartridge protocol (docs/transport.txt) for
// TypeScript cartridges on Bun or Node. Copy this file into a cartridge.

import { AsyncLocalStorage } from "node:async_hooks";
import { chmodSync } from "node:fs";
import net from "node:net";
import { randomBytes } from "node:crypto";

type Json = any;
type Handler = (data: Json) => Promise<Json>;
type Address = { cartridge: string; socket: string; token: string };
type Grant = { from: string; scope: "call" | "event"; names: string[] };
export type Directory = {
	needs: Record<string, Address>;
	events: Record<string, Address[]>;
	accept: Record<string, Grant>;
	host?: Address;
};

const UNAUTHORIZED = -32001;
const NOT_PROVIDED = -32002;
const HISTORY = 1024;

const traces = new AsyncLocalStorage<string | undefined>();

/** The trace of the request this code is handling. */
export function turn(): string | undefined { return traces.getStore(); }

/** One diagnostic line on stderr; the host records it. */
export function diagnostic(src: string, msg: string, fields: Record<string, unknown> = {}): void {
	process.stderr.write(JSON.stringify({ t: Date.now(), turn: turn() ?? null, src, msg, ...fields }) + "\n");
}

class RpcError extends Error {
	constructor(public code: number, message: string) { super(message); }
}

/** One JSON-RPC connection: concurrent requests out, requests and notifications in. */
class Peer {
	private next = 0;
	private buffer = "";
	private pending = new Map<number, { resolve: (v: Json) => void; reject: (e: Error) => void }>();
	closed = false;
	onRequest: (method: string, params: Json, reply: (result: Json, error?: RpcError) => void) => void = (_m, _p, reply) =>
		reply(null, new RpcError(-32601, "no requests are served here"));
	onNotification: (method: string, params: Json) => void = () => {};
	onClose: () => void = () => {};

	constructor(private socket: net.Socket) {
		socket.setEncoding("utf8");
		socket.on("data", chunk => this.receive(String(chunk)));
		socket.on("close", () => this.close());
		socket.on("error", () => this.close());
	}

	private receive(chunk: string) {
		this.buffer += chunk;
		let at: number;
		while ((at = this.buffer.indexOf("\n")) >= 0) {
			const line = this.buffer.slice(0, at).trim();
			this.buffer = this.buffer.slice(at + 1);
			if (!line) continue;
			let frame: any;
			try { frame = JSON.parse(line); } catch { this.write({ jsonrpc: "2.0", id: null, error: { code: -32700, message: "parse error" } }); continue; }
			if (typeof frame.method === "string") {
				if (frame.id === undefined || frame.id === null) { this.onNotification(frame.method, frame.params ?? null); continue; }
				let answered = false;
				this.onRequest(frame.method, frame.params ?? null, (result, error) => {
					if (answered) return;
					answered = true;
					this.write(error
						? { jsonrpc: "2.0", id: frame.id, error: { code: error.code, message: error.message } }
						: { jsonrpc: "2.0", id: frame.id, result: result ?? null });
				});
				continue;
			}
			const waiter = typeof frame.id === "number" ? this.pending.get(frame.id) : undefined;
			if (!waiter) continue;
			this.pending.delete(frame.id);
			frame.error ? waiter.reject(new RpcError(frame.error.code, frame.error.message)) : waiter.resolve(frame.result ?? null);
		}
	}

	private write(frame: Json) { if (!this.closed) this.socket.write(JSON.stringify(frame) + "\n"); }

	call(method: string, params: Json): Promise<Json> {
		if (this.closed) return Promise.reject(new Error("connection closed"));
		const id = ++this.next;
		return new Promise((resolve, reject) => {
			this.pending.set(id, { resolve, reject });
			this.write({ jsonrpc: "2.0", id, method, params });
		});
	}

	notify(method: string, params: Json): boolean {
		if (this.closed) return false;
		this.write({ jsonrpc: "2.0", method, params });
		return true;
	}

	close() {
		if (this.closed) return;
		this.closed = true;
		for (const waiter of this.pending.values()) waiter.reject(new Error("connection closed"));
		this.pending.clear();
		this.socket.destroy();
		this.onClose();
	}
}

type Channel = { seq: number; history: Json[]; subscribers: Set<Peer> };

export class Wire {
	private host = process.env.CARTRIDGE_HOST_TOKEN ?? "";
	private timeout = Number(process.env.CARTRIDGE_CONNECT_TIMEOUT_SECS ?? 30) * 1000;
	private directory: Directory = { needs: {}, events: {}, accept: {} };
	private services = new Map<string, Handler>();
	private events = new Map<string, Handler>();
	private channels = new Map<string, Channel>();
	private clients = new Map<string, Promise<Peer>>();
	private finalizers: (() => void | Promise<void>)[] = [];
	private server: net.Server;
	private stopped = false;
	name = "";

	constructor() {
		const socket = process.env.CARTRIDGE_SOCKET;
		if (!socket || !this.host) {
			process.stderr.write("CARTRIDGE_SOCKET and CARTRIDGE_HOST_TOKEN must be set; a cartridge is started by the cartridge host\n");
			process.exit(2);
		}
		this.server = net.createServer(connection => this.accept(new Peer(connection)));
		this.server.listen(socket, () => { try { chmodSync(socket, 0o600); } catch {} });
		process.stdin.on("end", () => this.close());
		process.stdin.on("close", () => this.close());
		process.stdin.resume();
	}

	/** `apply` receives the config; any other name serves that provided key. */
	on(name: string, handler: Handler): void { this.services.set(name, handler); }

	/** Handle an event this cartridge declares in `on`; a non-null return is its answer. */
	listen(name: string, handler: Handler): void { this.events.set(name, handler); }

	/** Runs when the cartridge stops, last registered first; the process exits once every cleanup settles. */
	onClose(cleanup: () => void | Promise<void>): void { this.finalizers.push(cleanup); }

	needs(): string[] { return Object.keys(this.directory.needs); }

	private accept(peer: Peer) {
		let grant: Grant | "host" | undefined;
		peer.onRequest = (method, params, reply) => {
			if (!grant) {
				if (method !== "auth") return reply(null, new RpcError(UNAUTHORIZED, "authenticate first")), peer.close();
				const token = String(params?.token ?? "");
				grant = token === this.host ? "host" : this.directory.accept[token];
				if (!grant) return reply(null, new RpcError(UNAUTHORIZED, "unknown token")), peer.close();
				return reply({ cartridge: this.name });
			}
			void this.handle(peer, grant, method, params, reply);
		};
		peer.onClose = () => { for (const channel of this.channels.values()) channel.subscribers.delete(peer); };
	}

	private async handle(peer: Peer, grant: Grant | "host", method: string, params: Json, reply: (r: Json, e?: RpcError) => void) {
		const host = grant === "host";
		const allows = (scope: "call" | "event", name: string) => host || ((grant as Grant).scope === scope && (grant as Grant).names.includes(name));
		const run = (handler: Handler, data: Json) => traces.run(typeof params?.trace === "string" ? params.trace : undefined, () => handler(data));
		try {
			switch (method) {
				case "apply": {
					if (!host) throw new RpcError(UNAUTHORIZED, "`apply` is not granted to this token");
					this.name = String(params?.name ?? "");
					this.directory = params?.directory ?? this.directory;
					const apply = this.services.get("apply");
					if (apply) await apply(params?.config ?? null);
					return reply({});
				}
				case "directory":
					if (!host) throw new RpcError(UNAUTHORIZED, "`directory` is not granted to this token");
					this.directory = params?.directory ?? this.directory;
					return reply({});
				case "dispose":
					if (!host) throw new RpcError(UNAUTHORIZED, "`dispose` is not granted to this token");
					reply({});
					return this.close();
				case "call": {
					const key = String(params?.key ?? "");
					if (!allows("call", key)) throw new RpcError(UNAUTHORIZED, `\`${key}\` is not granted to this token`);
					const service = key === "apply" ? undefined : this.services.get(key);
					if (!service) throw new RpcError(NOT_PROVIDED, `\`${key}\` is not provided`);
					return reply(await run(service, params?.args ?? null));
				}
				case "event": {
					const name = String(params?.name ?? "");
					if (!allows("event", name)) throw new RpcError(UNAUTHORIZED, `\`${name}\` is not granted to this token`);
					const listener = this.events.get(name);
					if (!listener) return reply(null);
					try { return reply(await run(listener, params?.data ?? null)); }
					catch (error) { this.publishKind("error", "error", { event: name, error: String(error) }); throw error; }
				}
				case "subscribe": {
					if (!host && (grant as Grant).scope !== "call") throw new RpcError(UNAUTHORIZED, "`subscribe` is not granted to this token");
					const channel = this.channel(String(params?.channel ?? ""));
					const since = typeof params?.since === "number" ? params.since : undefined;
					if (since !== undefined) for (const envelope of channel.history) if (envelope.seq > since) peer.notify("channel", envelope);
					channel.subscribers.add(peer);
					return reply({ seq: channel.seq });
				}
				case "unsubscribe":
					this.channel(String(params?.channel ?? "")).subscribers.delete(peer);
					return reply({});
				default:
					throw new RpcError(-32601, `unknown method \`${method}\``);
			}
		} catch (error) {
			if (error instanceof RpcError) return reply(null, error);
			const detail = error instanceof Error ? error.stack ?? error.message : String(error);
			return reply(null, new RpcError(-32000, detail));
		}
	}

	private channel(name: string): Channel {
		let channel = this.channels.get(name);
		if (!channel) { channel = { seq: 0, history: [], subscribers: new Set() }; this.channels.set(name, channel); }
		return channel;
	}

	private publishKind(name: string, kind: string, data: Json) {
		const channel = this.channel(name);
		const envelope = { channel: name, seq: ++channel.seq, kind, data };
		channel.history.push(envelope);
		if (channel.history.length > HISTORY) channel.history.shift();
		for (const peer of channel.subscribers) if (!peer.notify("channel", envelope)) channel.subscribers.delete(peer);
	}

	publish(channel: string, data: Json): void { this.publishKind(channel, "data", data); }

	private client(address: Address): Promise<Peer> {
		const existing = this.clients.get(address.token);
		if (existing) return existing.then(peer => peer.closed ? this.reconnect(address) : peer);
		return this.reconnect(address);
	}

	private reconnect(address: Address): Promise<Peer> {
		const connecting = (async () => {
			const deadline = Date.now() + this.timeout;
			for (let delay = 20; ; delay = Math.min(delay * 2, 500)) {
				try {
					const socket = await new Promise<net.Socket>((resolve, reject) => {
						const s = net.connect(address.socket, () => resolve(s));
						s.once("error", reject);
					});
					const peer = new Peer(socket);
					await peer.call("auth", { token: address.token });
					return peer;
				} catch (error) {
					if (error instanceof RpcError && error.code === UNAUTHORIZED) throw error;
					if (Date.now() >= deadline) throw new Error(`${address.cartridge} is unreachable: ${error}`);
					await new Promise(resolve => setTimeout(resolve, delay));
				}
			}
		})();
		this.clients.set(address.token, connecting);
		connecting.catch(() => this.clients.delete(address.token));
		return connecting;
	}

	/** Call a key this cartridge needs. */
	async call(key: string, args: Json): Promise<Json> {
		const address = this.directory.needs[key];
		if (!address) throw new Error(`\`${key}\` is not a need of this cartridge`);
		return (await this.client(address)).call("call", { key, args, trace: turn() });
	}

	private async event(address: Address, name: string, data: Json): Promise<Json> {
		return (await this.client(address)).call("event", { name, data, trace: turn() });
	}

	/** Send an event to every listener without waiting. */
	emit(name: string, data: Json): void {
		for (const address of this.directory.events[name] ?? []) void this.event(address, name, data).catch(() => {});
	}

	/** The first non-null answer, asking listeners in order. */
	async bail(name: string, data: Json): Promise<Json> {
		for (const address of this.directory.events[name] ?? []) {
			const answer = await this.event(address, name, data);
			if (answer !== null && answer !== undefined) return answer;
		}
		return null;
	}

	/** Every non-null answer, with the cartridge that gave it. */
	async gather(name: string, data: Json): Promise<{ from: string; data: Json }[]> {
		const listeners = this.directory.events[name] ?? [];
		const answers = await Promise.allSettled(listeners.map(address => this.event(address, name, data)));
		return answers.flatMap((answer, i) => answer.status === "fulfilled" && answer.value !== null ? [{ from: listeners[i].cartridge, data: answer.value }] : []);
	}

	/** Emit and publish on the channel of the same name. */
	notify(name: string, data: Json): void { this.emit(name, data); this.publish(name, data); }

	/** Ask the host: `status`, `snapshot`, `cartridges`, `bridge.status`, `bridge.call`. */
	async ask(method: string, params: Json = null): Promise<Json> {
		const address = this.directory.host;
		if (!address) throw new Error("no host address in the directory");
		return (await this.client(address)).call(method, params);
	}

	close(): void {
		if (this.stopped) return;
		this.stopped = true;
		this.server.close();
		void (async () => {
			for (const cleanup of this.finalizers.reverse()) { try { await cleanup(); } catch {} }
			for (const client of this.clients.values()) await client.then(peer => peer.close()).catch(() => {});
			setTimeout(() => process.exit(0), 50);
		})();
	}
}

export function token(): string { return randomBytes(32).toString("hex"); }
