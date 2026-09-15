#![allow(unused)]
use super::*;

mod tests {
	use super::*;

	#[test]
	fn io_error_into_codec_is_a_decode_carrying_the_original_message() {
		let io = std::io::Error::new(std::io::ErrorKind::BrokenPipe, "pipe is gone");
		let codec: CodecError = io.into();
		assert!(matches!(codec, CodecError::Decode(_)));
		let shown = codec.to_string();
		assert!(
			shown.starts_with("codec decode:"),
			"displayed as a decode error: {shown}"
		);
		assert!(
			shown.contains("pipe is gone"),
			"original io message survives: {shown}"
		);
	}

	#[test]
	fn serde_error_into_codec_preserves_the_serde_message() {
		let serde_err = serde_json::from_str::<serde_json::Value>("{ not json").unwrap_err();
		let original = serde_err.to_string();
		let codec: CodecError = serde_err.into();
		assert!(matches!(codec, CodecError::Decode(_)));
		assert!(
			codec.to_string().contains(&original),
			"serde message preserved"
		);
	}

	#[test]
	fn rpc_error_absorbs_adapter_and_codec_via_from() {
		let a: RpcError = AdapterError::Eof.into();
		assert!(matches!(a, RpcError::Adapter(_)));
		assert!(a.to_string().contains("eof"), "{a}");

		let c: RpcError = CodecError::Encode("bad frame".into()).into();
		assert!(matches!(c, RpcError::Codec(_)));
		assert!(c.to_string().contains("bad frame"), "{c}");
	}
}
mod tests_2 {
	use super::*;
	use tokio::io::{AsyncReadExt, AsyncWriteExt};

	#[tokio::test]
	async fn inproc_reader_drains_leftover_across_small_reads() {
		let (a, b) = InprocAdapter::pair();
		let (_ar, mut aw) = Box::new(a).split();
		let (mut br, _bw) = Box::new(b).split();

		aw.write_all(b"hello").await.unwrap();

		let mut got = Vec::new();
		let mut chunk = [0u8; 2];
		while got.len() < 5 {
			let n = br.read(&mut chunk).await.unwrap();
			assert!(n > 0, "reader makes progress");
			got.extend_from_slice(&chunk[..n]);
		}
		assert_eq!(&got, b"hello", "leftover bytes are drained across reads");
	}
}
mod tests_3 {
	use super::*;
	use serde_json::json;

	#[test]
	fn json_roundtrip_single_frame() {
		let mut c = JsonEnvelopeCodec::new();
		let mut buf = BytesMut::new();
		c.encode(json!({"id": 1, "method": "ping"}), &mut buf)
			.unwrap();
		let got = c.decode(&mut buf).unwrap().expect("one frame");
		assert_eq!(got, json!({"id": 1, "method": "ping"}));
		assert!(c.decode(&mut buf).unwrap().is_none());
	}

	#[test]
	fn json_decodes_multiple_frames_from_one_buffer() {
		let mut c = JsonEnvelopeCodec::new();
		let mut buf = BytesMut::new();
		c.encode(json!({"a": 1}), &mut buf).unwrap();
		c.encode(json!({"b": 2}), &mut buf).unwrap();
		assert_eq!(c.decode(&mut buf).unwrap().unwrap(), json!({"a": 1}));
		assert_eq!(c.decode(&mut buf).unwrap().unwrap(), json!({"b": 2}));
		assert!(c.decode(&mut buf).unwrap().is_none());
	}

	#[test]
	fn json_tolerates_crlf_and_skips_blank_lines() {
		let mut c = JsonEnvelopeCodec::new();
		let mut buf = BytesMut::from(&b"\n\r\n{\"ok\":true}\r\n"[..]);
		let got = c.decode(&mut buf).unwrap().expect("frame after blanks");
		assert_eq!(got, json!({"ok": true}));
		assert!(c.decode(&mut buf).unwrap().is_none());
	}

	#[test]
	fn json_many_consecutive_newlines_do_not_overflow() {
		let mut c = JsonEnvelopeCodec::new();
		let mut bytes = vec![b'\n'; 100_000];
		bytes.extend_from_slice(b"{\"v\":42}\n");
		let mut buf = BytesMut::from(&bytes[..]);
		assert_eq!(c.decode(&mut buf).unwrap().unwrap(), json!({"v": 42}));
	}

	#[test]
	fn json_partial_line_yields_none_until_newline() {
		let mut c = JsonEnvelopeCodec::new();
		let mut buf = BytesMut::from(&b"{\"partial\":1}"[..]);
		assert!(
			c.decode(&mut buf).unwrap().is_none(),
			"incomplete line -> None"
		);
		buf.extend_from_slice(b"\n");
		assert_eq!(c.decode(&mut buf).unwrap().unwrap(), json!({"partial": 1}));
	}
}
mod tests_4 {
	use super::Channel;
	use super::InprocAdapter;
	use serde_json::json;

	#[tokio::test]
	async fn channel_roundtrip_json_envelope() {
		let (a, b) = InprocAdapter::pair();
		let mut ca = Channel::new(a);
		let mut cb = Channel::new(b);
		ca.send(json!({"hello": "world"})).await.unwrap();
		let got = cb.recv().await.unwrap().unwrap();
		assert_eq!(got["hello"], "world");
	}

	#[tokio::test]
	async fn recv_returns_none_on_closed_adapter() {
		let (a, b) = InprocAdapter::pair();
		let ca = Channel::new(a);
		let mut cb = Channel::new(b);
		drop(ca);
		assert!(cb.recv().await.unwrap().is_none(), "EOF -> Ok(None)");
	}
}
mod cwd_tag_tests {
	use super::*;

	#[test]
	fn path_tag_is_stable_and_nonempty() {
		let dir = std::env::current_dir().unwrap();
		let a = path_tag(&dir);
		let b = path_tag(&dir);
		assert_eq!(a, b, "same path must yield the same tag");
		assert_eq!(a.len(), 16, "tag is 16 hex chars");
		assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
	}
}

#[test]
fn a_root_that_does_not_exist_yet_tags_the_same_from_every_spelling() {
	let tmp = tempfile::tempdir().unwrap();
	let root = std::fs::canonicalize(tmp.path()).unwrap();
	std::fs::create_dir_all(root.join("sibling")).unwrap();

	let absent = root.join("store");
	assert!(
		!absent.exists(),
		"the leaf must be absent for this to mean anything"
	);
	let round_about = root.join("sibling").join("..").join("store");

	assert_eq!(
		path_tag(&absent),
		path_tag(&round_about),
		"two spellings of one absent root must tag the same, or a daemon and its client miss each other"
	);
}

#[cfg(unix)]
#[test]
fn the_tag_is_the_same_before_and_after_the_root_is_created() {
	let tmp = tempfile::tempdir().unwrap();
	let real = tmp.path().join("real");
	std::fs::create_dir_all(&real).unwrap();
	let through = tmp.path().join("link");
	std::os::unix::fs::symlink(&real, &through).unwrap();
	let dir = through.join("store");
	assert_ne!(
		through,
		std::fs::canonicalize(&through).unwrap(),
		"this test needs a root whose spelling differs from its canonical form"
	);

	let before = path_tag(&dir);
	std::fs::create_dir_all(&dir).unwrap();
	let after = path_tag(&dir);

	assert_eq!(
		before, after,
		"a root created between two processes must not move the socket under them"
	);
}
