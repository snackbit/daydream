use super::*;
use std::io::{Read, Write};
fn boundary(status: u16, response: String, sse: bool) -> (String, std::thread::JoinHandle<Value>) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}/v1", listener.local_addr().unwrap());
    let thread = std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut bytes = Vec::new();
        let mut buffer = [0; 8192];
        let (offset, size) = loop {
            let count = socket.read(&mut buffer).unwrap();
            assert!(count > 0);
            bytes.extend_from_slice(&buffer[..count]);
            if let Some(end) = bytes.windows(4).position(|v| v == b"\r\n\r\n") {
                let headers = std::str::from_utf8(&bytes[..end])
                    .unwrap()
                    .to_ascii_lowercase();
                let size: usize = headers
                    .lines()
                    .find_map(|line| line.strip_prefix("content-length:"))
                    .unwrap()
                    .trim()
                    .parse()
                    .unwrap();
                break (end + 4, size);
            }
        };
        while bytes.len() < offset + size {
            let count = socket.read(&mut buffer).unwrap();
            assert!(count > 0);
            bytes.extend_from_slice(&buffer[..count]);
        }
        let body = serde_json::from_slice(&bytes[offset..offset + size]).unwrap();
        write!(socket,"HTTP/1.1 {status} Synthetic\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{response}",if sse{"text/event-stream"}else{"application/json"},response.len()).unwrap();
        body
    });
    (base, thread)
}
fn route(subscription: bool) -> Route {
    Route {
        protocol: Protocol::Responses,
        token_limit: TokenLimit::Modern,
        subscription_id: subscription.then(|| "00000000-0000-4000-8000-000000000001".into()),
        subscription,
    }
}
fn body() -> Value {
    json!({"model":"synthetic-reasoning","temperature":0,"max_tokens":8192,"response_format":{"type":"json_object"},"messages":[{"role":"system","content":"Synthetic JSON instruction"},{"role":"user","content":"Synthetic text"}]})
}
fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
}
#[test]
fn authenticated_completion_runs_on_the_native_worker_stack() {
    let (base, thread) = boundary(200, json!({"status":"completed","output":[{"type":"message","content":[{"type":"output_text","text":"{\"ok\":true}"}]}]}).to_string(), false);
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .unwrap();
    let result = runtime
        .block_on(async move {
            tokio::spawn(async move {
                super::super::subscription::diagnostics::operation(
                    "model_test",
                    None,
                    None,
                    async {
                        authenticated_completion(
                            None,
                            &base,
                            None,
                            Some(&route(false)),
                            body(),
                            128000,
                            &AtomicBool::new(false),
                        )
                        .await
                    },
                )
                .await
            })
            .await
            .unwrap()
        })
        .unwrap();
    assert_eq!(result["choices"][0]["message"]["content"], "{\"ok\":true}");
    thread.join().unwrap();
}
fn completed_event() -> String {
    format!(
        "event: response.completed\ndata: {}\n\n",
        json!({"type":"response.completed","response":{"status":"completed","output":[{"type":"message","content":[{"type":"output_text","text":"{\"ok\":true}"}]}]}})
    )
}
fn subscription_result(response: String) -> (Result<Value, String>, Value) {
    let (base, thread) = boundary(200, response, true);
    let result = runtime().block_on(completion(
        &base,
        Some("synthetic-oauth"),
        Some(&route(true)),
        body(),
        128000,
        &AtomicBool::new(false),
    ));
    (result, thread.join().unwrap())
}
#[test]
fn subscription_accepts_keepalive_before_completed_output() {
    let (result, body) = subscription_result(format!(
        ": synthetic heartbeat\n\nevent: keepalive\ndata: \n\n{}data: [DONE]\n\n",
        completed_event()
    ));
    assert_eq!(
        result.unwrap()["choices"][0]["message"]["content"],
        "{\"ok\":true}"
    );
    assert_eq!(body["text"]["format"]["type"], "json_object");
}
#[test]
fn subscription_accepts_multiline_event_data() {
    let response = completed_event().replace(",\"type\"", ",\ndata: \"type\"");
    let (result, _) = subscription_result(response);
    assert_eq!(
        result.unwrap()["choices"][0]["message"]["content"],
        "{\"ok\":true}"
    );
}
#[test]
fn subscription_retains_streamed_text_when_completed_output_is_sparse() {
    let response = format!(
        "data: {}\n\ndata: {}\n\n",
        json!({"type":"response.output_text.delta","output_index":0,"content_index":0,"delta":"{\"ok\":true}"}),
        json!({"type":"response.completed","response":{"status":"completed","output":[]}})
    );
    let (result, _) = subscription_result(response);
    assert_eq!(
        result.unwrap()["choices"][0]["message"]["content"],
        "{\"ok\":true}"
    );
}
#[test]
fn incomplete_and_malformed_streams_do_not_return_partial_results() {
    for (response, expected) in [
        ("data: {\"type\":\"response.output_text.delta\",\"output_index\":0,\"content_index\":0,\"delta\":\"synthetic-private-partial\"}\n\n".into(), "model_response_incomplete"),
        ("data: {synthetic-private-malformed}\n\n".into(), "model_response_invalid"),
        ("data: {\"type\":\"response.incomplete\",\"response\":{\"status\":\"incomplete\"}}\n\n".into(), "model_response_incomplete"),
        ("data: [DONE]\n\n".into(), "model_response_incomplete"),
    ] {
        let (result, _) = subscription_result(response);
        assert_eq!(result.unwrap_err(), expected);
    }
}
#[test]
fn subscription_orders_text_parts_and_does_not_duplicate_done_text() {
    let events = [
        json!({"type":"response.output_text.delta","output_index":1,"content_index":0,"delta":"true}"}),
        json!({"type":"response.output_text.delta","output_index":0,"content_index":0,"delta":"{\"ok\":"}),
        json!({"type":"response.output_text.done","output_index":0,"content_index":0,"text":"{\"ok\":"}),
        json!({"type":"response.completed","response":{"status":"completed","output":[]}}),
    ];
    let response = events
        .iter()
        .map(|event| format!("data: {event}\n\n"))
        .collect();
    let (result, _) = subscription_result(response);
    assert_eq!(
        result.unwrap()["choices"][0]["message"]["content"],
        "{\"ok\":true}"
    );
}
#[test]
fn invalid_provider_json_logs_the_request_and_decode_stage_without_the_body() {
    use tracing::instrument::WithSubscriber;
    use tracing::Instrument;
    let capture = crate::telemetry::test_support::LogCapture::default();
    let request_id = uuid::Uuid::new_v4();
    let (base, thread) = boundary(200, "data: {synthetic-private-content}\n\n".into(), true);
    let error = runtime()
        .block_on(
            async {
                completion(
                    &base,
                    Some("synthetic-private-credential"),
                    Some(&route(true)),
                    body(),
                    128000,
                    &AtomicBool::new(false),
                )
                .instrument(tracing::info_span!("model_test", %request_id))
                .await
            }
            .with_subscriber(capture.subscriber()),
        )
        .unwrap_err();
    thread.join().unwrap();
    assert_eq!(error, "model_response_invalid");
    let logs = capture.text();
    assert!(logs.contains(&request_id.to_string()));
    assert!(logs.contains("sse_json"));
    assert!(logs.contains("model_json_invalid"));
    assert!(logs.contains("category"));
    assert!(!logs.contains("synthetic-private"));
}
#[test]
fn subscription_rejects_text_over_the_limit_and_cancellation_prevents_transport() {
    let (base, thread) = boundary(200, completed_event(), true);
    let result = runtime().block_on(completion(
        &base,
        None,
        Some(&route(true)),
        body(),
        8,
        &AtomicBool::new(false),
    ));
    thread.join().unwrap();
    assert_eq!(result.unwrap_err(), "model_response_limit");
    let result = runtime().block_on(completion(
        "http://127.0.0.1:9",
        None,
        Some(&route(true)),
        body(),
        128000,
        &AtomicBool::new(true),
    ));
    assert_eq!(result.unwrap_err(), "model_cancelled");
}
#[test]
fn subscription_handles_utf8_bom_crlf_and_vision_input() {
    let (base, thread) = boundary(
        200,
        format!("\u{feff}{}", completed_event().replace('\n', "\r\n")),
        true,
    );
    let body = json!({"model":"synthetic-vision","messages":[{"role":"user","content":[{"type":"text","text":"Synthetic image"},{"type":"image_url","image_url":{"url":"data:image/png;base64,c3ludGhldGlj"}}]}]});
    let result = runtime()
        .block_on(completion(
            &base,
            None,
            Some(&route(true)),
            body,
            128000,
            &AtomicBool::new(false),
        ))
        .unwrap();
    assert_eq!(result["choices"][0]["message"]["content"], "{\"ok\":true}");
    let request = thread.join().unwrap();
    assert_eq!(request["input"][0]["content"][1]["type"], "input_image");
}
#[test]
fn responses_omit_reasoning_incompatible_parameters_and_validate_completed_output() {
    let (base,thread)=boundary(200,json!({"status":"completed","output":[{"type":"message","content":[{"type":"output_text","text":"{\"ok\":true}"}]}]}).to_string(),false);
    let result = runtime()
        .block_on(completion(
            &base,
            Some("synthetic-key"),
            Some(&route(false)),
            body(),
            128000,
            &AtomicBool::new(false),
        ))
        .unwrap();
    assert_eq!(result["choices"][0]["message"]["content"], "{\"ok\":true}");
    let body = thread.join().unwrap();
    assert_eq!(body["max_output_tokens"], 8192);
    assert_eq!(body["store"], false);
    assert!(body.get("temperature").is_none());
    assert!(body.get("max_tokens").is_none());
    assert!(body.get("messages").is_none());
}
#[test]
fn subscription_stream_never_commits_partial_text_or_falls_back_to_api_billing() {
    let response="event: response.output_text.delta\ndata: {\"type\":\"response.output_text.delta\",\"output_index\":0,\"content_index\":0,\"delta\":\"Synthetic partial\"}\n\nevent: response.failed\ndata: {\"type\":\"response.failed\",\"response\":{\"error\":{\"code\":\"subscription_sharing_usage_limit_exceeded\"}}}\n\n".to_owned();
    let (base, thread) = boundary(200, response, true);
    assert_eq!(
        runtime()
            .block_on(completion(
                &base,
                Some("synthetic-oauth"),
                Some(&route(true)),
                body(),
                128000,
                &AtomicBool::new(false)
            ))
            .unwrap_err(),
        "model_subscription_usage_limit"
    );
    let body = thread.join().unwrap();
    assert_eq!(body["stream"], true);
    assert_eq!(body["store"], false);
    for field in [
        "max_output_tokens",
        "max_tokens",
        "temperature",
        "response_format",
    ] {
        assert!(body.get(field).is_none());
    }
}
#[test]
fn subscription_failure_codes_and_parameters_are_logged_without_provider_messages() {
    use tracing::instrument::WithSubscriber;
    let capture = crate::telemetry::test_support::LogCapture::default();
    let response = format!(
        "data: {}\n\n",
        json!({"type":"error","code":"subscription_sharing_unsupported_capability","param":"temperature","message":"synthetic-private-provider-message"})
    );
    let (base, thread) = boundary(200, response, true);
    let result = runtime().block_on(
        completion(
            &base,
            None,
            Some(&route(true)),
            body(),
            128000,
            &AtomicBool::new(false),
        )
        .with_subscriber(capture.subscriber()),
    );
    thread.join().unwrap();
    assert_eq!(result.unwrap_err(), "model_capability_unsupported");
    let logs = capture.text();
    assert!(logs.contains("subscription_sharing_unsupported_capability"));
    assert!(logs.contains("temperature"));
    assert!(!logs.contains("synthetic-private"));
}
#[test]
fn provider_errors_and_malformed_success_bodies_are_private_and_bounded() {
    for (status, response, expected) in [
        (
            500,
            "Synthetic private provider body".into(),
            "model_server_failed",
        ),
        (
            200,
            "Synthetic malformed private JSON".into(),
            "model_response_invalid",
        ),
        (
            200,
            json!({"synthetic":"x".repeat(300)}).to_string(),
            "model_response_limit",
        ),
    ] {
        let (base, thread) = boundary(status, response, false);
        let error = runtime()
            .block_on(completion(
                &base,
                None,
                None,
                body(),
                128,
                &AtomicBool::new(false),
            ))
            .unwrap_err();
        assert_eq!(error, expected);
        thread.join().unwrap();
    }
}
#[test]
fn missing_subscription_cannot_use_an_api_key() {
    let mut route = route(true);
    route.subscription_id = None;
    let error = runtime()
        .block_on(authenticated_completion(
            None,
            "https://api.openai.com/v1",
            Some("synthetic-api-key"),
            Some(&route),
            body(),
            128,
            &AtomicBool::new(false),
        ))
        .unwrap_err();
    assert_eq!(error, "model_subscription_reconnect");
}
#[test]
fn subscription_credentials_cannot_be_sent_to_compatible_endpoints() {
    let error = runtime()
        .block_on(authenticated_completion(
            None,
            "http://127.0.0.1:9",
            None,
            Some(&route(true)),
            body(),
            128,
            &AtomicBool::new(false),
        ))
        .unwrap_err();
    assert_eq!(error, "model_subscription_invalid");
}
