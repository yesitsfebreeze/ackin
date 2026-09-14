//! Typed request/response channels over any adapter: the Adapter seam, the
//! newline-JSON codec, their errors, the channel pairing them, and the local
//! endpoints (per-user unix sockets, Windows named pipes) processes find each
//! other with. [`crate::transport::rpc::Peer`] speaks through this without
//! knowing the wire.

// ==== [error] ====

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AdapterError {
	#[error("adapter i/o: {0}")]
	Io(#[from] std::io::Error),
	#[error("adapter eof")]
	Eof,
	#[error("adapter codec: {0}")]
	Codec(#[from] CodecError),
	// A peer answered and refused this caller.
	#[error("adapter unauthenticated: {0}")]
	Unauthenticated(String),
	// This caller refused the endpoint before contacting it.
	#[error("adapter untrusted endpoint: {0}")]
	UntrustedEndpoint(String),
	#[error("adapter: {0}")]
	Other(String),
}

#[derive(Debug, Error)]
pub enum CodecError {
	#[error("codec encode: {0}")]
	Encode(String),
	#[error("codec decode: {0}")]
	Decode(String),
}

#[derive(Debug, Error)]
pub enum RpcError {
	#[error("rpc adapter: {0}")]
	Adapter(String),
	#[error("rpc codec: {0}")]
	Codec(String),
	#[error("rpc application error: {0}")]
	Application(String),
}

impl From<serde_json::Error> for CodecError {
	fn from(e: serde_json::Error) -> Self {
		CodecError::Decode(e.to_string())
	}
}

impl From<std::io::Error> for CodecError {
	fn from(e: std::io::Error) -> Self {
		CodecError::Decode(format!("io: {e}"))
	}
}

impl From<AdapterError> for RpcError {
	fn from(e: AdapterError) -> Self {
		RpcError::Adapter(e.to_string())
	}
}

impl From<CodecError> for RpcError {
	fn from(e: CodecError) -> Self {
		RpcError::Codec(e.to_string())
	}
}

#[cfg(test)]
#[path = "tests/typed.rs"]
mod typed_tests;

// ==== [adapter] ====

use std::pin::Pin;
use std::task::{Context as TaskContext, Poll};

use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::sync::mpsc;

pub type DynRead = Box<dyn AsyncRead + Unpin + Send>;
pub type DynWrite = Box<dyn AsyncWrite + Unpin + Send>;

pub trait Adapter: Send + 'static {
	fn split(self: Box<Self>) -> (DynRead, DynWrite);
}

pub struct InprocAdapter {
	reader: InprocReader,
	writer: InprocWriter,
}

impl InprocAdapter {
	pub fn pair() -> (Self, Self) {
		let (a_to_b_tx, a_to_b_rx) = mpsc::unbounded_channel::<Vec<u8>>();
		let (b_to_a_tx, b_to_a_rx) = mpsc::unbounded_channel::<Vec<u8>>();
		let a = InprocAdapter {
			reader: InprocReader::new(b_to_a_rx),
			writer: InprocWriter::new(a_to_b_tx),
		};
		let b = InprocAdapter {
			reader: InprocReader::new(a_to_b_rx),
			writer: InprocWriter::new(b_to_a_tx),
		};
		(a, b)
	}
}

impl Adapter for InprocAdapter {
	fn split(self: Box<Self>) -> (DynRead, DynWrite) {
		(Box::new(self.reader), Box::new(self.writer))
	}
}

pub struct InprocReader {
	rx: mpsc::UnboundedReceiver<Vec<u8>>,
	leftover: Vec<u8>,
}

impl InprocReader {
	fn new(rx: mpsc::UnboundedReceiver<Vec<u8>>) -> Self {
		Self {
			rx,
			leftover: Vec::new(),
		}
	}
}

impl AsyncRead for InprocReader {
	fn poll_read(
		mut self: Pin<&mut Self>,
		cx: &mut TaskContext<'_>,
		buf: &mut ReadBuf<'_>,
	) -> Poll<std::io::Result<()>> {
		if !self.leftover.is_empty() {
			let n = std::cmp::min(self.leftover.len(), buf.remaining());
			let tail = self.leftover.split_off(n);
			buf.put_slice(&self.leftover);
			self.leftover = tail;
			return Poll::Ready(Ok(()));
		}
		match self.rx.poll_recv(cx) {
			Poll::Ready(Some(bytes)) => {
				let n = std::cmp::min(bytes.len(), buf.remaining());
				buf.put_slice(&bytes[..n]);
				if n < bytes.len() {
					self.leftover.extend_from_slice(&bytes[n..]);
				}
				Poll::Ready(Ok(()))
			}
			Poll::Ready(None) => Poll::Ready(Ok(())),
			Poll::Pending => Poll::Pending,
		}
	}
}

pub struct InprocWriter {
	tx: mpsc::UnboundedSender<Vec<u8>>,
}

impl InprocWriter {
	fn new(tx: mpsc::UnboundedSender<Vec<u8>>) -> Self {
		Self { tx }
	}
}

impl AsyncWrite for InprocWriter {
	fn poll_write(
		self: Pin<&mut Self>,
		_cx: &mut TaskContext<'_>,
		buf: &[u8],
	) -> Poll<std::io::Result<usize>> {
		self.tx
			.send(buf.to_vec())
			.map_err(|_| std::io::Error::new(std::io::ErrorKind::BrokenPipe, "inproc closed"))?;
		Poll::Ready(Ok(buf.len()))
	}
	fn poll_flush(self: Pin<&mut Self>, _cx: &mut TaskContext<'_>) -> Poll<std::io::Result<()>> {
		Poll::Ready(Ok(()))
	}
	fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut TaskContext<'_>) -> Poll<std::io::Result<()>> {
		Poll::Ready(Ok(()))
	}
}

// ==== [codec] ====

use bytes::BytesMut;
use serde_json::Value;
use tokio_util::codec::{Decoder, Encoder};

// One JSON value per line. `with_max` caps a frame for servers that read from unauthenticated peers.
#[derive(Default)]
pub struct JsonEnvelopeCodec {
	max: Option<usize>,
}

impl JsonEnvelopeCodec {
	pub fn new() -> Self {
		Self { max: None }
	}

	/// A codec that refuses any frame longer than `max` bytes.
	pub fn with_max(max: usize) -> Self {
		Self { max: Some(max) }
	}
}

impl Encoder<Value> for JsonEnvelopeCodec {
	type Error = CodecError;

	fn encode(&mut self, frame: Value, dst: &mut BytesMut) -> Result<(), CodecError> {
		let s = serde_json::to_string(&frame).map_err(|e| CodecError::Encode(e.to_string()))?;
		if s.contains('\n') {
			return Err(CodecError::Encode("frame contained newline".into()));
		}
		dst.extend_from_slice(s.as_bytes());
		dst.extend_from_slice(b"\n");
		Ok(())
	}
}

impl Decoder for JsonEnvelopeCodec {
	type Item = Value;
	type Error = CodecError;

	fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Value>, CodecError> {
		// Loop, NOT recursion, over leading blank lines — N consecutive newlines
		// once recursed N deep (see json_many_consecutive_newlines_do_not_overflow).
		loop {
			let nl = src.iter().position(|&b| b == b'\n');
			let too_long = |len: usize| self.max.is_some_and(|max| len > max);
			let Some(pos) = nl else {
				if too_long(src.len()) {
					return Err(CodecError::Decode("frame exceeds the size limit".into()));
				}
				return Ok(None);
			};
			if too_long(pos) {
				return Err(CodecError::Decode("frame exceeds the size limit".into()));
			}
			let line = src.split_to(pos + 1);
			let slice = &line[..pos];
			let trimmed = if slice.last() == Some(&b'\r') {
				&slice[..slice.len() - 1]
			} else {
				slice
			};
			if trimmed.is_empty() {
				continue;
			}
			let v: Value =
				serde_json::from_slice(trimmed).map_err(|e| CodecError::Decode(e.to_string()))?;
			return Ok(Some(v));
		}
	}
}

// ==== [channel] ====

use futures::{SinkExt, StreamExt};
use tokio_util::codec::{FramedRead, FramedWrite};

pub struct Channel {
	reader: FramedRead<DynRead, JsonEnvelopeCodec>,
	writer: FramedWrite<DynWrite, JsonEnvelopeCodec>,
}

impl Channel {
	pub fn new<A: Adapter>(adapter: A) -> Self {
		Self::with_codec(adapter, JsonEnvelopeCodec::new())
	}

	/// A channel that refuses incoming frames longer than `max` bytes.
	pub fn with_max_frame<A: Adapter>(adapter: A, max: usize) -> Self {
		Self::with_codec(adapter, JsonEnvelopeCodec::with_max(max))
	}

	fn with_codec<A: Adapter>(adapter: A, codec: JsonEnvelopeCodec) -> Self {
		let (read_half, write_half) = Box::new(adapter).split();
		let reader = FramedRead::new(read_half, codec);
		let writer = FramedWrite::new(write_half, JsonEnvelopeCodec::new());
		Self { reader, writer }
	}

	/// The two halves, for a reader and a writer that run on different tasks.
	pub fn into_split(
		self,
	) -> (
		FramedRead<DynRead, JsonEnvelopeCodec>,
		FramedWrite<DynWrite, JsonEnvelopeCodec>,
	) {
		(self.reader, self.writer)
	}

	pub async fn send(&mut self, frame: Value) -> Result<(), AdapterError> {
		self.writer
			.send(frame)
			.await
			.map_err(adapter_err_from_codec)?;
		Ok(())
	}

	pub async fn recv(&mut self) -> Result<Option<Value>, AdapterError> {
		match self.reader.next().await {
			Some(Ok(f)) => Ok(Some(f)),
			Some(Err(e)) => Err(adapter_err_from_codec(e)),
			None => Ok(None),
		}
	}
}

fn adapter_err_from_codec(e: CodecError) -> AdapterError {
	AdapterError::Codec(e)
}

// ==== [local] ====

#[cfg(unix)]
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub enum Endpoint {
	#[cfg(unix)]
	Unix(PathBuf),
	#[cfg(windows)]
	NamedPipe(String),
}

// Unix `sun_path` ceiling — a path at or past this breaks the bind with
// "path must be shorter than SUN_LEN"; conservative by design.
#[cfg(unix)]
const SUN_LEN_MAX: usize = 100;

impl Endpoint {
	/// The endpoint `<prefix>-<tag>` for a root directory: every process that
	/// names the same root, under any spelling, resolves the same socket.
	pub fn for_root(prefix: &str, root: &std::path::Path) -> Self {
		Self::scoped(&format!("{prefix}-{}", path_tag(root)))
	}

	// Reconstruct from the wire form produced by `display()`.
	pub fn parse(s: &str) -> Self {
		#[cfg(unix)]
		{
			Endpoint::Unix(PathBuf::from(s))
		}
		#[cfg(windows)]
		{
			Endpoint::NamedPipe(s.to_string())
		}
	}

	/// A per-user endpoint named `name`: `$XDG_RUNTIME_DIR/<name>.sock` when that
	/// path is short enough, `/tmp/<name>-<user>.sock` otherwise.
	pub fn scoped(name: &str) -> Self {
		#[cfg(unix)]
		{
			let user = std::env::var("USER").unwrap_or_else(|_| "default".into());
			let fallback = PathBuf::from(format!("/tmp/{name}-{user}.sock"));
			let path = std::env::var_os("XDG_RUNTIME_DIR")
				.map(PathBuf::from)
				.map(|d| d.join(format!("{name}.sock")))
				// Fall back to /tmp when XDG_RUNTIME_DIR would exceed SUN_LEN.
				.filter(|p| p.as_os_str().len() < SUN_LEN_MAX)
				.unwrap_or(fallback);
			Endpoint::Unix(path)
		}
		#[cfg(windows)]
		{
			let user = std::env::var("USERNAME").unwrap_or_else(|_| "default".into());
			Endpoint::NamedPipe(format!(r"\\.\pipe\{name}-{user}"))
		}
	}

	pub fn display(&self) -> String {
		match self {
			#[cfg(unix)]
			Endpoint::Unix(p) => p.display().to_string(),
			#[cfg(windows)]
			Endpoint::NamedPipe(n) => n.clone(),
		}
	}
}

// FNV-1a over the canonical path: stable across processes, unlike DefaultHasher.
fn canonical_or_parent(dir: &std::path::Path) -> std::path::PathBuf {
	if let Ok(c) = dir.canonicalize() {
		return c;
	}
	match (dir.parent(), dir.file_name()) {
		(Some(parent), Some(leaf)) => match parent.canonicalize() {
			Ok(c) => c.join(leaf),
			Err(_) => dir.to_path_buf(),
		},
		_ => dir.to_path_buf(),
	}
}

pub fn path_tag(dir: &std::path::Path) -> String {
	let canon = canonical_or_parent(dir);
	let s = canon.to_string_lossy();
	let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
	for b in s.as_bytes() {
		hash ^= *b as u64;
		hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
	}
	format!("{hash:016x}")
}

#[cfg(unix)]
pub struct UnixStreamAdapter {
	stream: tokio::net::UnixStream,
}

#[cfg(unix)]
impl UnixStreamAdapter {
	pub fn new(stream: tokio::net::UnixStream) -> Self {
		Self { stream }
	}
	pub async fn connect(path: &Path) -> Result<Self, AdapterError> {
		let stream = tokio::net::UnixStream::connect(path).await?;
		Ok(Self { stream })
	}
}

#[cfg(unix)]
impl Adapter for UnixStreamAdapter {
	fn split(self: Box<Self>) -> (DynRead, DynWrite) {
		let (r, w) = self.stream.into_split();
		(Box::new(r), Box::new(w))
	}
}

#[cfg(windows)]
pub struct NamedPipeAdapter {
	inner: NamedPipeInner,
}

#[cfg(windows)]
enum NamedPipeInner {
	Server(tokio::net::windows::named_pipe::NamedPipeServer),
	Client(tokio::net::windows::named_pipe::NamedPipeClient),
}

#[cfg(windows)]
impl NamedPipeAdapter {
	pub fn from_server(server: tokio::net::windows::named_pipe::NamedPipeServer) -> Self {
		Self {
			inner: NamedPipeInner::Server(server),
		}
	}
	pub async fn connect(pipe_name: &str) -> Result<Self, AdapterError> {
		let client = tokio::net::windows::named_pipe::ClientOptions::new().open(pipe_name)?;
		Ok(Self {
			inner: NamedPipeInner::Client(client),
		})
	}
}

#[cfg(windows)]
impl Adapter for NamedPipeAdapter {
	fn split(self: Box<Self>) -> (DynRead, DynWrite) {
		match self.inner {
			NamedPipeInner::Server(s) => {
				let (r, w) = tokio::io::split(s);
				(Box::new(r), Box::new(w))
			}
			NamedPipeInner::Client(c) => {
				let (r, w) = tokio::io::split(c);
				(Box::new(r), Box::new(w))
			}
		}
	}
}

pub enum LocalAdapter {
	#[cfg(unix)]
	Unix(UnixStreamAdapter),
	#[cfg(windows)]
	NamedPipe(NamedPipeAdapter),
}

impl Adapter for LocalAdapter {
	fn split(self: Box<Self>) -> (DynRead, DynWrite) {
		match *self {
			#[cfg(unix)]
			LocalAdapter::Unix(a) => Box::new(a).split(),
			#[cfg(windows)]
			LocalAdapter::NamedPipe(a) => Box::new(a).split(),
		}
	}
}

// The socket path and its target must belong to this user; checked before connecting.
#[cfg(unix)]
fn require_owned_by_caller(path: &Path) -> Result<(), AdapterError> {
	use std::os::unix::fs::{FileTypeExt, MetadataExt};
	let untrusted =
		|what: &str| AdapterError::UntrustedEndpoint(format!("{}: {what}", path.display()));
	// SAFETY: `geteuid` cannot fail and touches no memory the caller owns.
	let euid = unsafe { libc::geteuid() };
	let link = std::fs::symlink_metadata(path).map_err(|e| {
		if e.kind() == std::io::ErrorKind::NotFound {
			AdapterError::Io(e)
		} else {
			untrusted(&format!("cannot stat: {e}"))
		}
	})?;
	let target = std::fs::metadata(path).map_err(|e| untrusted(&format!("cannot resolve: {e}")))?;
	if link.uid() != euid {
		return Err(untrusted(&format!(
			"owned by uid {}, not {euid}",
			link.uid()
		)));
	}
	if target.uid() != euid {
		return Err(untrusted(&format!(
			"resolves to a path owned by uid {}, not {euid}",
			target.uid()
		)));
	}
	if !target.file_type().is_socket() {
		return Err(untrusted("not a socket"));
	}
	Ok(())
}

// SO_PEERCRED of this connection, which a swapped path cannot fake.
#[cfg(unix)]
fn require_peer_is_caller(adapter: &UnixStreamAdapter, path: &Path) -> Result<(), AdapterError> {
	// SAFETY: `geteuid` cannot fail and touches no memory the caller owns.
	require_peer_uid(adapter, path, unsafe { libc::geteuid() })
}

// The expected uid is a parameter so tests can exercise the refusal.
#[cfg(unix)]
fn require_peer_uid(
	adapter: &UnixStreamAdapter,
	path: &Path,
	expected: u32,
) -> Result<(), AdapterError> {
	let untrusted =
		|what: &str| AdapterError::UntrustedEndpoint(format!("{}: {what}", path.display()));
	let cred = adapter
		.stream
		.peer_cred()
		.map_err(|e| untrusted(&format!("cannot read peer credentials: {e}")))?;
	if cred.uid() != expected {
		return Err(untrusted(&format!(
			"served by uid {}, not {expected}",
			cred.uid()
		)));
	}
	Ok(())
}

pub async fn connect(endpoint: &Endpoint) -> Result<LocalAdapter, AdapterError> {
	match endpoint {
		#[cfg(unix)]
		Endpoint::Unix(path) => {
			require_owned_by_caller(path)?;
			let adapter = UnixStreamAdapter::connect(path).await?;
			require_peer_is_caller(&adapter, path)?;
			Ok(LocalAdapter::Unix(adapter))
		}
		#[cfg(windows)]
		Endpoint::NamedPipe(name) => Ok(LocalAdapter::NamedPipe(
			NamedPipeAdapter::connect(name).await?,
		)),
	}
}

// Test seam: connect expecting a uid that is not the server's.
#[cfg(unix)]
#[cfg(test)]
async fn connect_with_peer(
	endpoint: &Endpoint,
	expected_uid: u32,
) -> Result<LocalAdapter, AdapterError> {
	match endpoint {
		Endpoint::Unix(path) => {
			require_owned_by_caller(path)?;
			let adapter = UnixStreamAdapter::connect(path).await?;
			require_peer_uid(&adapter, path, expected_uid)?;
			Ok(LocalAdapter::Unix(adapter))
		}
		#[cfg(windows)]
		Endpoint::NamedPipe(name) => Ok(LocalAdapter::NamedPipe(
			NamedPipeAdapter::connect(name).await?,
		)),
	}
}

pub enum BindOutcome {
	Bound(LocalListener),
	AlreadyRunning,
}

#[derive(Debug, thiserror::Error)]
pub enum BindError {
	#[error("bind: {0}")]
	Io(#[from] std::io::Error),
	// Something this euid does not own holds the name.
	#[error("bind refused: {0}")]
	Untrusted(String),
}

// Owner-only permissions on the socket file.
#[cfg(unix)]
fn harden_socket(path: &Path) -> std::io::Result<()> {
	use std::os::unix::fs::PermissionsExt;
	std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
}

// The kernel creates the socket `0777 & ~umask`; narrowing the umask across the
// bind closes the window before `harden_socket` — 0o077 strips group and other
// only, so a directory created inside the window keeps its owner execute bit.
// Every umask write in the process takes this lock and restores what it read.
#[cfg(unix)]
static UMASK: parking_lot::Mutex<()> = parking_lot::Mutex::new(());

#[cfg(unix)]
fn bind_owner_only(path: &Path) -> std::io::Result<tokio::net::UnixListener> {
	let _guard = UMASK.lock();
	// SAFETY: `umask` cannot fail and touches no memory.
	let previous = unsafe { libc::umask(0o077) };
	let bound = tokio::net::UnixListener::bind(path);
	// SAFETY: restoring the value `umask` just returned.
	unsafe { libc::umask(previous) };
	bound
}

// Owner-only security descriptor for named pipes: one ACE for this process's SID.
#[cfg(windows)]
mod owner_only {
	use std::io;

	use windows_sys::Win32::Foundation::{CloseHandle, LocalFree, HANDLE};
	use windows_sys::Win32::Security::Authorization::{
		ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW,
		SDDL_REVISION_1,
	};
	use windows_sys::Win32::Security::{
		GetTokenInformation, TokenUser, PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES, TOKEN_QUERY,
		TOKEN_USER,
	};
	use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

	/// An owner-only security descriptor, freed on drop.
	pub struct OwnerOnlySd(PSECURITY_DESCRIPTOR);

	// SAFETY: the pointer is owned, never mutated after construction, and only
	// read through `attributes()`.
	unsafe impl Send for OwnerOnlySd {}
	unsafe impl Sync for OwnerOnlySd {}

	impl OwnerOnlySd {
		pub fn new() -> io::Result<Self> {
			let sid = current_user_sid()?;
			let sddl: Vec<u16> = format!("D:P(A;;GA;;;{sid})")
				.encode_utf16()
				.chain(std::iter::once(0))
				.collect();
			let mut psd: PSECURITY_DESCRIPTOR = std::ptr::null_mut();
			// SAFETY: `sddl` is NUL-terminated and outlives the call; `psd` receives
			// a LocalAlloc'd descriptor this value then owns.
			let ok = unsafe {
				ConvertStringSecurityDescriptorToSecurityDescriptorW(
					sddl.as_ptr(),
					SDDL_REVISION_1,
					&mut psd,
					std::ptr::null_mut(),
				)
			};
			if ok == 0 || psd.is_null() {
				return Err(io::Error::last_os_error());
			}
			Ok(Self(psd))
		}

		pub fn attributes(&self) -> SECURITY_ATTRIBUTES {
			SECURITY_ATTRIBUTES {
				nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
				lpSecurityDescriptor: self.0,
				bInheritHandle: 0,
			}
		}
	}

	impl Drop for OwnerOnlySd {
		fn drop(&mut self) {
			// SAFETY: allocated by ConvertStringSecurityDescriptorToSecurityDescriptorW
			// (LocalAlloc) and freed nowhere else.
			unsafe { LocalFree(self.0.cast()) };
		}
	}

	fn current_user_sid() -> io::Result<String> {
		let mut token: HANDLE = std::ptr::null_mut();
		// SAFETY: the pseudo-handle from GetCurrentProcess needs no close; `token`
		// receives a real handle closed below.
		if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) } == 0 {
			return Err(io::Error::last_os_error());
		}
		let out = token_user_sid(token);
		// SAFETY: `token` was opened here and is not used after this.
		unsafe { CloseHandle(token) };
		out
	}

	fn token_user_sid(token: HANDLE) -> io::Result<String> {
		let mut len: u32 = 0;
		// SAFETY: the sizing call is *expected* to fail; it only writes `len`.
		unsafe { GetTokenInformation(token, TokenUser, std::ptr::null_mut(), 0, &mut len) };
		if len == 0 {
			return Err(io::Error::last_os_error());
		}
		let mut buf = vec![0u8; len as usize];
		// SAFETY: `buf` is exactly the length the sizing call asked for.
		if unsafe { GetTokenInformation(token, TokenUser, buf.as_mut_ptr().cast(), len, &mut len) }
			== 0
		{
			return Err(io::Error::last_os_error());
		}
		// SAFETY: the buffer now holds a TOKEN_USER whose `Sid` points inside it.
		let sid = unsafe { (*buf.as_ptr().cast::<TOKEN_USER>()).User.Sid };
		let mut raw: *mut u16 = std::ptr::null_mut();
		// SAFETY: `sid` is valid for the lifetime of `buf`; `raw` receives a
		// LocalAlloc'd NUL-terminated string freed below.
		if unsafe { ConvertSidToStringSidW(sid, &mut raw) } == 0 || raw.is_null() {
			return Err(io::Error::last_os_error());
		}
		let mut n = 0usize;
		// SAFETY: walking a NUL-terminated buffer the call above guaranteed.
		while unsafe { *raw.add(n) } != 0 {
			n += 1;
		}
		// SAFETY: `raw[..n]` is the string body, exclusive of the terminator.
		let s = String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(raw, n) });
		// SAFETY: `raw` came from ConvertSidToStringSidW and is dead after this.
		unsafe { LocalFree(raw.cast()) };
		Ok(s)
	}
}

#[cfg(windows)]
fn create_pipe_instance(
	name: &str,
	sd: &owner_only::OwnerOnlySd,
	first: bool,
) -> std::io::Result<tokio::net::windows::named_pipe::NamedPipeServer> {
	use tokio::net::windows::named_pipe::ServerOptions;
	let mut attrs = sd.attributes();
	// SAFETY: `attrs` lives across the call and points at a descriptor `sd` owns.
	unsafe {
		ServerOptions::new()
			.first_pipe_instance(first)
			.create_with_security_attributes_raw(
				name,
				std::ptr::addr_of_mut!(attrs).cast::<std::ffi::c_void>(),
			)
	}
}

// The expected peer uid is a parameter so tests can reach the refusal.
#[cfg(unix)]
async fn bind_unix(path: &Path, expected_peer: u32) -> Result<BindOutcome, BindError> {
	let listener = match bind_owner_only(path) {
		Ok(listener) => listener,
		Err(e) if e.kind() != std::io::ErrorKind::AddrInUse => {
			return Err(e.into());
		}
		Err(_) => {
			// The name is taken: refuse unless it is ours and either answers as ours or is stale.
			require_owned_by_caller(path).map_err(|e| BindError::Untrusted(e.to_string()))?;
			match UnixStreamAdapter::connect(path).await {
				Ok(adapter) => {
					require_peer_uid(&adapter, path, expected_peer)
						.map_err(|e| BindError::Untrusted(e.to_string()))?;
					return Ok(BindOutcome::AlreadyRunning);
				}
				// Nothing answers a name we own: our own stale socket, ours to reclaim.
				Err(_) => {
					let _ = std::fs::remove_file(path);
					bind_owner_only(path)?
				}
			}
		}
	};
	harden_socket(path)?;
	Ok(BindOutcome::Bound(LocalListener {
		inner: listener,
		socket_path: path.to_path_buf(),
	}))
}

pub async fn bind(endpoint: &Endpoint) -> Result<BindOutcome, BindError> {
	match endpoint {
		#[cfg(unix)]
		Endpoint::Unix(path) => {
			// SAFETY: `geteuid` cannot fail and touches no memory the caller owns.
			bind_unix(path, unsafe { libc::geteuid() }).await
		}
		#[cfg(windows)]
		Endpoint::NamedPipe(name) => {
			// Fail closed: no descriptor, no pipe.
			let security = owner_only::OwnerOnlySd::new()?;
			match create_pipe_instance(name, &security, true) {
                Ok(server) => Ok(BindOutcome::Bound(LocalListener {
                    pipe_name: name.clone(),
                    security,
                    current: Some(server),
                })),
                Err(e)
                    if e.kind() == std::io::ErrorKind::PermissionDenied
                        || e.raw_os_error() == Some(5)    // ERROR_ACCESS_DENIED
                        || e.raw_os_error() == Some(231)  // ERROR_PIPE_BUSY
                =>
                {
                    Ok(BindOutcome::AlreadyRunning)
                }
                Err(e) => Err(e.into()),
            }
		}
	}
}

/// Adopt fd 0 as an already-bound listener handed over by a predecessor process.
#[cfg(unix)]
pub fn adopt(endpoint: &Endpoint) -> Result<LocalListener, BindError> {
	use std::os::fd::FromRawFd;
	let Endpoint::Unix(path) = endpoint;
	// SAFETY: the takeover contract places the listener at fd 0.
	let std_listener = unsafe { std::os::unix::net::UnixListener::from_raw_fd(0) };
	std_listener.set_nonblocking(true)?;
	let inner = tokio::net::UnixListener::from_std(std_listener)?;
	Ok(LocalListener {
		inner,
		socket_path: path.clone(),
	})
}

pub struct LocalListener {
	#[cfg(unix)]
	inner: tokio::net::UnixListener,
	#[cfg(unix)]
	socket_path: PathBuf,
	#[cfg(windows)]
	pipe_name: String,
	// Kept for the life of the listener: `accept` creates the *next* instance,
	// and an instance without this descriptor is a hole beside a locked door.
	#[cfg(windows)]
	security: owner_only::OwnerOnlySd,
	#[cfg(windows)]
	current: Option<tokio::net::windows::named_pipe::NamedPipeServer>,
}

#[cfg(unix)]
impl LocalListener {
	/// A close-on-exec dup of the listening fd, for handing to a successor process.
	pub fn dup_fd(&self) -> std::io::Result<std::os::fd::OwnedFd> {
		use std::os::fd::AsFd;
		self.inner.as_fd().try_clone_to_owned()
	}
}

impl LocalListener {
	// The expected uid is a parameter so tests can reach the refusal.
	#[cfg(unix)]
	async fn accept_from(&mut self, expected: u32) -> Result<LocalAdapter, std::io::Error> {
		loop {
			let (stream, _peer) = self.inner.accept().await?;
			match stream.peer_cred() {
				Ok(cred) if cred.uid() == expected => {
					return Ok(LocalAdapter::Unix(UnixStreamAdapter::new(stream)))
				}
				// Logged and dropped, not returned: both accept loops end on an
				// error, and a stranger must not stop a listener by knocking.
				Ok(cred) => tracing::warn!(
					target: "cartridge",
					"{}: refused a connection from uid {}, not {expected}",
					self.socket_path.display(),
					cred.uid()
				),
				Err(error) => tracing::warn!(
					target: "cartridge",
					"{}: refused a connection whose credentials do not read: {error}",
					self.socket_path.display()
				),
			}
		}
	}

	pub async fn accept(&mut self) -> Result<LocalAdapter, std::io::Error> {
		#[cfg(unix)]
		{
			// SAFETY: `geteuid` cannot fail and touches no memory the caller owns.
			self.accept_from(unsafe { libc::geteuid() }).await
		}
		#[cfg(windows)]
		{
			let server = self.current.take().expect("listener uninitialised");
			server.connect().await?;
			// Pre-create the next instance so subsequent accept doesn't race
			// a fast reconnect — with the same descriptor as the first.
			self.current = Some(create_pipe_instance(
				&self.pipe_name,
				&self.security,
				false,
			)?);
			Ok(LocalAdapter::NamedPipe(NamedPipeAdapter::from_server(
				server,
			)))
		}
	}
}

#[cfg(unix)]
impl Drop for LocalListener {
	fn drop(&mut self) {
		// Best-effort cleanup so the next daemon doesn't trip the stale-sock probe.
		let _ = std::fs::remove_file(&self.socket_path);
	}
}

#[cfg(all(test, windows))]
#[path = "tests/bind_windows.rs"]
mod bind_tests_windows;

#[cfg(all(test, unix))]
#[path = "tests/bind_unix.rs"]
mod bind_tests_unix;

#[cfg(all(test, unix))]
#[path = "tests/owner_unix.rs"]
mod owner_tests_unix;
