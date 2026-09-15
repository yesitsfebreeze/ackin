use super::*;
use crate::transport::typed::InprocAdapter;
use std::time::Duration;

fn server(adapter: InprocAdapter) -> Peer {
	let (peer, mut incoming) = Peer::spawn(adapter, None);
	tokio::spawn(async move {
		while let Some(message) = incoming.recv().await {
			let Incoming::Request(request) = message else {
				continue;
			};
			tokio::spawn(async move {
				match request.method.as_str() {
					"echo" => {
						let params = request.params.clone();
						request.reply(Ok(params));
					}
					"slow" => {
						tokio::time::sleep(Duration::from_millis(50)).await;
						request.reply(Ok(json!("slow")));
					}
					"fail" => request.reply(Err(Error::application("it broke"))),
					"drop" => drop(request),
					_ => request.reply(Err(Error::new(METHOD_NOT_FOUND, "unknown"))),
				}
			});
		}
	});
	peer
}

#[tokio::test]
async fn a_call_gets_its_result() {
	let (a, b) = InprocAdapter::pair();
	let _server = server(b);
	let (client, _) = Peer::spawn(a, None);
	assert_eq!(
		client.call("echo", json!({"x": 1})).await,
		Ok(json!({"x": 1}))
	);
}

#[tokio::test]
async fn calls_run_concurrently_and_match_by_id() {
	let (a, b) = InprocAdapter::pair();
	let _server = server(b);
	let (client, _) = Peer::spawn(a, None);
	let slow = client.call("slow", Value::Null);
	let fast = client.call("echo", json!(2));
	let fast = tokio::time::timeout(Duration::from_millis(40), fast).await;
	assert_eq!(
		fast.expect("the fast call must not wait for the slow one"),
		Ok(json!(2))
	);
	assert_eq!(slow.await, Ok(json!("slow")));
}

#[tokio::test]
async fn errors_cross_as_error_objects() {
	let (a, b) = InprocAdapter::pair();
	let _server = server(b);
	let (client, _) = Peer::spawn(a, None);
	let error = client.call("fail", Value::Null).await.unwrap_err();
	assert_eq!(error.code, APPLICATION_ERROR);
	assert_eq!(error.message, "it broke");
	let error = client.call("nope", Value::Null).await.unwrap_err();
	assert_eq!(error.code, METHOD_NOT_FOUND);
}

#[tokio::test]
async fn a_request_dropped_unanswered_is_answered_with_an_internal_error() {
	let (a, b) = InprocAdapter::pair();
	let _server = server(b);
	let (client, _) = Peer::spawn(a, None);
	let error = client.call("drop", Value::Null).await.unwrap_err();
	assert_eq!(error.code, INTERNAL_ERROR);
}

#[tokio::test]
async fn notifications_arrive_without_an_id() {
	let (a, b) = InprocAdapter::pair();
	let (_server, mut incoming) = Peer::spawn(b, None);
	let (client, _) = Peer::spawn(a, None);
	client.notify("ping", json!([1])).unwrap();
	match incoming.recv().await {
		Some(Incoming::Notification { method, params }) => {
			assert_eq!(method, "ping");
			assert_eq!(params, json!([1]));
		}
		_ => panic!("expected a notification"),
	}
}

#[tokio::test]
async fn closing_fails_waiting_calls() {
	let (a, b) = InprocAdapter::pair();
	let (server, _incoming) = Peer::spawn(b, None);
	let (client, _) = Peer::spawn(a, None);
	let call = tokio::spawn({
		let client = client.clone();
		async move { client.call("never", Value::Null).await }
	});
	tokio::time::sleep(Duration::from_millis(10)).await;
	server.close();
	let result = tokio::time::timeout(Duration::from_secs(1), call)
		.await
		.expect("the call must end when the other side closes")
		.unwrap();
	assert_eq!(result.unwrap_err().code, CLOSED);
	client.closed().await;
	assert!(client.is_closed());
}

#[tokio::test]
async fn dropping_the_last_handle_closes_the_connection() {
	let (a, b) = InprocAdapter::pair();
	let (server, _incoming) = Peer::spawn(b, None);
	let (client, _) = Peer::spawn(a, None);
	drop(client);
	tokio::time::timeout(Duration::from_secs(1), server.closed())
		.await
		.expect("the other side sees the close");
}

#[tokio::test]
async fn an_oversized_frame_closes_a_capped_connection() {
	let (a, b) = InprocAdapter::pair();
	let (server, _incoming) = Peer::spawn(b, Some(64));
	let (client, _) = Peer::spawn(a, None);
	client.notify("big", json!("x".repeat(200))).unwrap();
	tokio::time::timeout(Duration::from_secs(1), server.closed())
		.await
		.expect("an oversized frame closes the capped side");
}
