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
	#[error("adapter unauthenticated: {0}")]
	Unauthenticated(String),
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

#[derive(Default)]
pub struct JsonEnvelopeCodec {
	max: Option<usize>,
}

impl JsonEnvelopeCodec {
	pub fn new() -> Self {
		Self { max: None }
	}

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

	pub fn with_max_frame<A: Adapter>(adapter: A, max: usize) -> Self {
		Self::with_codec(adapter, JsonEnvelopeCodec::with_max(max))
	}

	fn with_codec<A: Adapter>(adapter: A, codec: JsonEnvelopeCodec) -> Self {
		let (read_half, write_half) = Box::new(adapter).split();
		let reader = FramedRead::new(read_half, codec);
		let writer = FramedWrite::new(write_half, JsonEnvelopeCodec::new());
		Self { reader, writer }
	}

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

use std::path::Path;
#[cfg(unix)]
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub enum Endpoint {
	#[cfg(unix)]
	Unix(PathBuf),
	#[cfg(windows)]
	NamedPipe(String),
}

impl Endpoint {
	pub fn local(path: &Path) -> Self {
		#[cfg(unix)]
		{
			Endpoint::Unix(path.to_path_buf())
		}
		#[cfg(windows)]
		{
			Endpoint::NamedPipe(format!(r"\\.\pipe\cartridge-{}", path_tag(path)))
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
	HandedServer(
		tokio::net::windows::named_pipe::NamedPipeServer,
		mpsc::UnboundedSender<tokio::net::windows::named_pipe::NamedPipeServer>,
	),
	Client(tokio::net::windows::named_pipe::NamedPipeClient),
}

#[cfg(windows)]
impl NamedPipeAdapter {
	pub fn from_server(server: tokio::net::windows::named_pipe::NamedPipeServer) -> Self {
		Self {
			inner: NamedPipeInner::Server(server),
		}
	}
	pub fn from_handed_server(
		server: tokio::net::windows::named_pipe::NamedPipeServer,
		returned: mpsc::UnboundedSender<tokio::net::windows::named_pipe::NamedPipeServer>,
	) -> Self {
		Self {
			inner: NamedPipeInner::HandedServer(server, returned),
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
			NamedPipeInner::HandedServer(s, tx) => {
				let (r, w) = tokio::io::split(s);
				let shared = std::sync::Arc::new(std::sync::Mutex::new(None));
				(
					Box::new(HandedRead {
						half: Some(r),
						shared: shared.clone(),
						returned: Some(tx.clone()),
					}),
					Box::new(HandedWrite {
						half: Some(w),
						shared,
						returned: Some(tx),
					}),
				)
			}
			NamedPipeInner::Client(c) => {
				let (r, w) = tokio::io::split(c);
				(Box::new(r), Box::new(w))
			}
		}
	}
}

/// The two split halves of a handed instance drop independently (they may
/// run on different tasks — see `Channel::into_split`), so whichever drops
/// second is the one that reunites them and hands the instance back.
#[cfg(windows)]
enum HandedHalf {
	Read(tokio::io::ReadHalf<tokio::net::windows::named_pipe::NamedPipeServer>),
	Write(tokio::io::WriteHalf<tokio::net::windows::named_pipe::NamedPipeServer>),
}

#[cfg(windows)]
type HandedSlot = std::sync::Arc<std::sync::Mutex<Option<HandedHalf>>>;

#[cfg(windows)]
struct HandedRead {
	half: Option<tokio::io::ReadHalf<tokio::net::windows::named_pipe::NamedPipeServer>>,
	shared: HandedSlot,
	returned: Option<mpsc::UnboundedSender<tokio::net::windows::named_pipe::NamedPipeServer>>,
}

#[cfg(windows)]
impl AsyncRead for HandedRead {
	fn poll_read(
		self: Pin<&mut Self>,
		cx: &mut TaskContext<'_>,
		buf: &mut ReadBuf<'_>,
	) -> Poll<std::io::Result<()>> {
		let this = self.get_mut();
		Pin::new(this.half.as_mut().expect("polled after drop")).poll_read(cx, buf)
	}
}

#[cfg(windows)]
impl Drop for HandedRead {
	fn drop(&mut self) {
		if let (Some(read), Some(tx)) = (self.half.take(), self.returned.take()) {
			reunite(HandedHalf::Read(read), &self.shared, &tx);
		}
	}
}

#[cfg(windows)]
struct HandedWrite {
	half: Option<tokio::io::WriteHalf<tokio::net::windows::named_pipe::NamedPipeServer>>,
	shared: HandedSlot,
	returned: Option<mpsc::UnboundedSender<tokio::net::windows::named_pipe::NamedPipeServer>>,
}

#[cfg(windows)]
impl AsyncWrite for HandedWrite {
	fn poll_write(
		self: Pin<&mut Self>,
		cx: &mut TaskContext<'_>,
		buf: &[u8],
	) -> Poll<std::io::Result<usize>> {
		let this = self.get_mut();
		Pin::new(this.half.as_mut().expect("polled after drop")).poll_write(cx, buf)
	}
	fn poll_flush(self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<std::io::Result<()>> {
		let this = self.get_mut();
		Pin::new(this.half.as_mut().expect("polled after drop")).poll_flush(cx)
	}
	fn poll_shutdown(self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<std::io::Result<()>> {
		let this = self.get_mut();
		Pin::new(this.half.as_mut().expect("polled after drop")).poll_shutdown(cx)
	}
}

#[cfg(windows)]
impl Drop for HandedWrite {
	fn drop(&mut self) {
		if let (Some(write), Some(tx)) = (self.half.take(), self.returned.take()) {
			reunite(HandedHalf::Write(write), &self.shared, &tx);
		}
	}
}

#[cfg(windows)]
fn reunite(
	mine: HandedHalf,
	shared: &HandedSlot,
	tx: &mpsc::UnboundedSender<tokio::net::windows::named_pipe::NamedPipeServer>,
) {
	let mut slot = shared.lock().unwrap();
	match (slot.take(), mine) {
		(Some(HandedHalf::Write(write)), HandedHalf::Read(read))
		| (Some(HandedHalf::Read(read)), HandedHalf::Write(write)) => {
			drop(slot);
			return_handed_instance(read.unsplit(write), tx);
		}
		(None, mine) => *slot = Some(mine),
		(Some(_), _) => unreachable!("one of each half per instance"),
	}
}

/// `disconnect()` discards what the client has not read yet (a final reply,
/// an auth refusal), so `FlushFileBuffers` waits for that read first — on an
/// OS thread, since the wait is unbounded and would stall a dropping runtime.
#[cfg(windows)]
fn return_handed_instance(
	server: tokio::net::windows::named_pipe::NamedPipeServer,
	tx: &mpsc::UnboundedSender<tokio::net::windows::named_pipe::NamedPipeServer>,
) {
	let tx = tx.clone();
	std::thread::spawn(move || {
		use std::os::windows::io::AsRawHandle;
		use windows_sys::Win32::Storage::FileSystem::FlushFileBuffers;
		// SAFETY: `server` is a live, open pipe handle for the duration of this call.
		unsafe {
			FlushFileBuffers(server.as_raw_handle() as _);
		}
		// mio reads ConnectNamedPipe's ERROR_PIPE_CONNECTED/ERROR_NO_DATA as
		// "already connected, Ok", so a failed disconnect would return an
		// instance that still looks connected and makes `accept` busy-loop on
		// it; losing the instance beats poisoning the pool with one like that.
		if let Err(e) = server.disconnect() {
			tracing::warn!(
				target: "cartridge",
				"dropping a pipe instance: disconnect failed: {e}"
			);
			return;
		}
		let _ = tx.send(server);
	});
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
	let target = std::fs::metadata(path).map_err(|e| {
		// A non-symlink path missing here just raced its own unlink (an honest
		// restart, not a substitution); a symlink missing its target is exactly
		// what `a_dangling_symlink_is_refused` catches.
		if e.kind() == std::io::ErrorKind::NotFound && !link.file_type().is_symlink() {
			AdapterError::Io(e)
		} else {
			untrusted(&format!("cannot resolve: {e}"))
		}
	})?;
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
		Endpoint::NamedPipe(name) => {
			let adapter = NamedPipeAdapter::connect(name).await?;
			require_pipe_served_by_caller(&adapter, name)?;
			Ok(LocalAdapter::NamedPipe(adapter))
		}
	}
}

// The pipe namespace is global, so an opened-but-unchecked pipe would send
// the auth token to whoever claimed the name first; checked here, after
// open, because a named pipe has no credential to read before then.
#[cfg(windows)]
fn require_pipe_served_by_caller(
	adapter: &NamedPipeAdapter,
	name: &str,
) -> Result<(), AdapterError> {
	use std::os::windows::io::AsRawHandle;
	use windows_sys::Win32::System::Pipes::GetNamedPipeServerProcessId;

	let untrusted = |what: &str| AdapterError::UntrustedEndpoint(format!("{name}: {what}"));
	let NamedPipeInner::Client(client) = &adapter.inner else {
		unreachable!("NamedPipeAdapter::connect always makes a Client");
	};
	let mut pid: u32 = 0;
	// SAFETY: `client` is a live, open handle for the duration of this call.
	let ok = unsafe { GetNamedPipeServerProcessId(client.as_raw_handle(), &mut pid) };
	if ok == 0 {
		return Err(untrusted(&format!(
			"cannot read who serves it: {}",
			std::io::Error::last_os_error()
		)));
	}
	let server_sid = owner_only::user_sid_of_process(pid)
		.map_err(|e| untrusted(&format!("cannot read the server's identity: {e}")))?;
	let our_sid = owner_only::current_user_sid()
		.map_err(|e| untrusted(&format!("cannot read our own identity: {e}")))?;
	if server_sid != our_sid {
		return Err(untrusted(&format!(
			"served by a different user ({server_sid})"
		)));
	}
	Ok(())
}

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
	#[error("bind refused: {0}")]
	Untrusted(String),
}

#[cfg(unix)]
fn harden_socket(path: &Path) -> std::io::Result<()> {
	use std::os::unix::fs::PermissionsExt;
	std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
}

// The kernel creates the socket `0777 & ~umask`, so narrowing the umask across
// the bind closes the window before `harden_socket` runs. Every umask write
// in the process takes this lock and restores what it read.
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
	use windows_sys::Win32::System::Threading::{
		GetCurrentProcess, OpenProcess, OpenProcessToken, PROCESS_QUERY_LIMITED_INFORMATION,
	};

	pub struct OwnerOnlySd(PSECURITY_DESCRIPTOR);

	// SAFETY: the pointer is owned, never mutated after construction, and only
	// read through `attributes()`.
	unsafe impl Send for OwnerOnlySd {}
	unsafe impl Sync for OwnerOnlySd {}

	impl OwnerOnlySd {
		pub fn new() -> io::Result<Self> {
			let sid = current_user_sid()?;
			Self::from_sddl(&format!("D:P(A;;GA;;;{sid})"))
		}

		/// A node's AppContainer has its own SID; without it on the descriptor
		/// too, the pipe its parent made for it is unreachable.
		pub fn shared_with(container: &str) -> io::Result<Self> {
			let user = current_user_sid()?;
			Self::from_sddl(&format!("D:P(A;;GA;;;{user})(A;;GA;;;{container})"))
		}

		fn from_sddl(sddl: &str) -> io::Result<Self> {
			let sddl: Vec<u16> = sddl.encode_utf16().chain(std::iter::once(0)).collect();
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
			self.attributes_with(false)
		}

		pub fn attributes_inheritable(&self) -> SECURITY_ATTRIBUTES {
			self.attributes_with(true)
		}

		fn attributes_with(&self, inherit: bool) -> SECURITY_ATTRIBUTES {
			SECURITY_ATTRIBUTES {
				nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
				lpSecurityDescriptor: self.0,
				bInheritHandle: i32::from(inherit),
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

	pub(super) fn current_user_sid() -> io::Result<String> {
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

	pub(super) fn user_sid_of_process(pid: u32) -> io::Result<String> {
		// SAFETY: `pid` is whatever the caller read off the connection; a
		// bad value just fails the call below, nothing unsafe about it.
		let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
		if process.is_null() {
			return Err(io::Error::last_os_error());
		}
		let mut token: HANDLE = std::ptr::null_mut();
		// SAFETY: `process` was just opened above and closed below either way.
		let opened = unsafe { OpenProcessToken(process, TOKEN_QUERY, &mut token) };
		// SAFETY: `process` is not used again after this.
		unsafe { CloseHandle(process) };
		if opened == 0 {
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

#[cfg(unix)]
async fn bind_unix(path: &Path, expected_peer: u32) -> Result<BindOutcome, BindError> {
	let listener = match bind_owner_only(path) {
		Ok(listener) => listener,
		Err(e) if e.kind() != std::io::ErrorKind::AddrInUse => {
			return Err(e.into());
		}
		Err(_) => {
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
		socket_dev: socket_identity(path).ok(),
	}))
}

#[cfg(windows)]
const ERROR_ACCESS_DENIED: i32 = 5;
#[cfg(windows)]
const ERROR_PIPE_BUSY: i32 = 231;

#[cfg(windows)]
pub const PIPE_INSTANCES: usize = 16;

#[cfg(windows)]
pub const PIPE_HANDLES_ENV: &str = "CARTRIDGE_PIPE_HANDLES";

#[cfg(windows)]
const PIPE_BUFFER: u32 = 64 * 1024;

/// A node's AppContainer is denied the pipe namespace outright (`Access is
/// denied` on any create), so its unconfined parent creates every instance
/// here and the node only ever inherits them.
#[cfg(windows)]
pub fn broker(endpoint: &Endpoint, container_sid: &str) -> Result<Vec<usize>, BindError> {
	use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
	use windows_sys::Win32::Storage::FileSystem::{
		FILE_FLAG_FIRST_PIPE_INSTANCE, FILE_FLAG_OVERLAPPED, PIPE_ACCESS_DUPLEX,
	};
	use windows_sys::Win32::System::Pipes::{
		CreateNamedPipeW, PIPE_READMODE_BYTE, PIPE_TYPE_BYTE, PIPE_WAIT,
	};

	let Endpoint::NamedPipe(name) = endpoint;
	let security = owner_only::OwnerOnlySd::shared_with(container_sid)?;
	let wide: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
	let mut handles = Vec::with_capacity(PIPE_INSTANCES);
	for index in 0..PIPE_INSTANCES {
		let attrs = security.attributes_inheritable();
		// Raw `CreateNamedPipeW`, not `ServerOptions`: tokio binds a handle it
		// creates to this process's completion port for good, and the node
		// must bind its own — registering here first answered `The parameter
		// is incorrect`.
		//
		// SAFETY: `wide` is NUL-terminated and `attrs` points at a descriptor
		// `security` owns; both outlive the call.
		let handle = unsafe {
			CreateNamedPipeW(
				wide.as_ptr(),
				PIPE_ACCESS_DUPLEX
					| FILE_FLAG_OVERLAPPED
					| if index == 0 {
						FILE_FLAG_FIRST_PIPE_INSTANCE
					} else {
						0
					},
				PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT,
				PIPE_INSTANCES as u32,
				PIPE_BUFFER,
				PIPE_BUFFER,
				0,
				&attrs,
			)
		};
		if handle == INVALID_HANDLE_VALUE || handle.is_null() {
			return Err(BindError::Io(std::io::Error::last_os_error()));
		}
		// Left open on purpose: the child inherits its own copy, and closing
		// the last handle to an instance takes the instance with it. They go
		// when this process exits, which is when the node's are gone too.
		handles.push(handle as usize);
	}
	Ok(handles)
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
			if let Some(listener) = adopt_handed(name)? {
				return Ok(BindOutcome::Bound(listener));
			}
			// Fail closed: no descriptor, no pipe.
			let security = owner_only::OwnerOnlySd::new()?;
			match create_pipe_instance(name, &security, true) {
				Ok(server) => {
					let (handed_tx, handed_rx) = mpsc::unbounded_channel();
					Ok(BindOutcome::Bound(LocalListener {
						pipe_name: name.clone(),
						security: Some(security),
						current: Some(server),
						handed: Vec::new(),
						handed_tx,
						handed_rx,
					}))
				}
				Err(e) if e.raw_os_error() == Some(ERROR_PIPE_BUSY) => {
					Ok(BindOutcome::AlreadyRunning)
				}
				// `FILE_FLAG_FIRST_PIPE_INSTANCE` answers ACCESS_DENIED both when
				// the name is already served and when this process (an
				// AppContainer) simply can't create pipes; conflating the two
				// turned a permission failure into false contention, so the name
				// is asked directly whether anything is actually there.
				Err(e)
					if e.raw_os_error() == Some(ERROR_ACCESS_DENIED)
						|| e.kind() == std::io::ErrorKind::PermissionDenied =>
				{
					match tokio::net::windows::named_pipe::ClientOptions::new().open(name) {
						Ok(_) => Ok(BindOutcome::AlreadyRunning),
						Err(busy) if busy.raw_os_error() == Some(ERROR_PIPE_BUSY) => {
							Ok(BindOutcome::AlreadyRunning)
						}
						// Nothing holds the name, so the refusal was about this
						// process, not a neighbour.
						Err(_) => Err(BindError::Untrusted(format!(
							"{name}: {e}, and nothing serves that name — this \
							 process may not create it"
						))),
					}
				}
				Err(e) => Err(e.into()),
			}
		}
	}
}

/// Read once: a second reader would take instances that are not its own.
#[cfg(windows)]
fn adopt_handed(name: &str) -> Result<Option<LocalListener>, BindError> {
	let Some(handed) = std::env::var_os(PIPE_HANDLES_ENV) else {
		return Ok(None);
	};
	std::env::remove_var(PIPE_HANDLES_ENV);
	let mut servers = Vec::new();
	for field in handed
		.to_string_lossy()
		.split(',')
		.filter(|f| !f.is_empty())
	{
		let handle: usize = field.parse().map_err(|_| {
			BindError::Untrusted(format!("{PIPE_HANDLES_ENV} is not a list of handles"))
		})?;
		// SAFETY: the value names a pipe instance this process inherited, and
		// each is taken once — the variable is removed above.
		let server = unsafe {
			tokio::net::windows::named_pipe::NamedPipeServer::from_raw_handle(handle as *mut _)
		}?;
		servers.push(server);
	}
	if servers.is_empty() {
		return Ok(None);
	}
	// Every instance goes in the pool `accept` waits on; one held back in
	// `current` would be an instance a client can reach and nobody serves.
	let (handed_tx, handed_rx) = mpsc::unbounded_channel();
	Ok(Some(LocalListener {
		pipe_name: name.to_owned(),
		security: None,
		current: None,
		handed: servers,
		handed_tx,
		handed_rx,
	}))
}

#[cfg(unix)]
fn socket_identity(path: &Path) -> std::io::Result<(u64, u64)> {
	use std::os::unix::fs::MetadataExt;
	let meta = std::fs::symlink_metadata(path)?;
	Ok((meta.dev(), meta.ino()))
}

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
		socket_dev: socket_identity(path).ok(),
	})
}

pub struct LocalListener {
	#[cfg(unix)]
	inner: tokio::net::UnixListener,
	#[cfg(unix)]
	socket_path: PathBuf,
	#[cfg(unix)]
	socket_dev: Option<(u64, u64)>,
	#[cfg(windows)]
	pipe_name: String,
	// `None` in a node: it was handed its instances and cannot make more, so
	// there is nothing to create the next one with. Otherwise kept for the
	// listener's life — `accept` uses it to create each next instance.
	#[cfg(windows)]
	security: Option<owner_only::OwnerOnlySd>,
	#[cfg(windows)]
	current: Option<tokio::net::windows::named_pipe::NamedPipeServer>,
	#[cfg(windows)]
	handed: Vec<tokio::net::windows::named_pipe::NamedPipeServer>,
	#[cfg(windows)]
	handed_tx: mpsc::UnboundedSender<tokio::net::windows::named_pipe::NamedPipeServer>,
	#[cfg(windows)]
	handed_rx: mpsc::UnboundedReceiver<tokio::net::windows::named_pipe::NamedPipeServer>,
}

#[cfg(unix)]
impl LocalListener {
	pub fn dup_fd(&self) -> std::io::Result<std::os::fd::OwnedFd> {
		use std::os::fd::AsFd;
		self.inner.as_fd().try_clone_to_owned()
	}
}

impl LocalListener {
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
			if let Some(security) = self.security.as_ref() {
				let server = self.current.take().expect("listener uninitialised");
				server.connect().await?;
				self.current = Some(create_pipe_instance(&self.pipe_name, security, false)?);
				return Ok(LocalAdapter::NamedPipe(NamedPipeAdapter::from_server(
					server,
				)));
			}
			// A node cannot make another instance to replace one, so a finished
			// connection goes back on `handed_rx` (see `split` on
			// `NamedPipeInner::HandedServer`) instead of being destroyed.
			loop {
				while let Ok(server) = self.handed_rx.try_recv() {
					self.handed.push(server);
				}
				if self.handed.is_empty() {
					// Blocking here (rather than erroring) keeps `serve` from
					// tearing live connections down over a queue about to
					// drain. `self.handed_tx` is a live sender this listener
					// holds, plus a clone in every outstanding `HandedServer`,
					// so the channel cannot close while this waits.
					let server = self
						.handed_rx
						.recv()
						.await
						.expect("the listener holds a sender, so the channel cannot close");
					self.handed.push(server);
					continue;
				}
				// Idle instances all listen at once and the kernel picks which
				// one a client lands on, so every idle instance is waited on
				// together rather than one at a time.
				let pending: Vec<_> = self
					.handed
					.iter()
					.map(|server| Box::pin(server.connect()))
					.collect();
				let (connected, index, rest) = futures::future::select_all(pending).await;
				// The rest borrow `handed`, and it is about to be taken from.
				drop(rest);
				connected?;
				let server = self.handed.remove(index);
				return Ok(LocalAdapter::NamedPipe(
					NamedPipeAdapter::from_handed_server(server, self.handed_tx.clone()),
				));
			}
		}
	}
}

#[cfg(unix)]
impl Drop for LocalListener {
	fn drop(&mut self) {
		// Only unlinks if the path still names this listener's socket: a
		// listener that outlives its stop can drop after a successor rebinds
		// the same name, and unlink-by-path would take that one instead.
		if self.socket_dev == socket_identity(&self.socket_path).ok() {
			let _ = std::fs::remove_file(&self.socket_path);
		}
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
