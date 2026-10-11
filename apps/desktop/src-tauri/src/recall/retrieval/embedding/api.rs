use super::EmbeddingPurpose;
use crate::settings::RecallEmbeddingSettings;
use serde::Deserialize;
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(test)]
use std::time::Duration;

#[derive(Deserialize)]
struct Response {
    data: Vec<Row>,
}
#[derive(Deserialize)]
struct Row {
    index: usize,
    embedding: Vec<f32>,
}

pub(super) async fn embed(
    settings: &RecallEmbeddingSettings,
    key: Option<&str>,
    purpose: EmbeddingPurpose,
    texts: &[String],
    cancelled: &AtomicBool,
) -> Result<Vec<Vec<f32>>, String> {
    if cancelled.load(Ordering::Acquire) {
        return Err("recall_cancelled".into());
    }
    if !settings.enabled {
        return Err("recall_embedding_remote_disabled".into());
    }
    let base = settings
        .base_url
        .as_deref()
        .ok_or("recall_embedding_endpoint_missing")?;
    let url = reqwest::Url::parse(base).map_err(|_| "recall_embedding_endpoint_invalid")?;
    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err("recall_embedding_endpoint_invalid".into());
    }
    let model = settings
        .model
        .as_deref()
        .filter(|model| !model.trim().is_empty())
        .ok_or("recall_embedding_model_missing")?;
    let input: Vec<String> = texts
        .iter()
        .map(|text| match (settings.convention.as_str(), purpose) {
            ("e5", EmbeddingPurpose::Query) => format!("query: {text}"),
            ("e5", EmbeddingPurpose::Document) => format!("passage: {text}"),
            ("bge", EmbeddingPurpose::Query) => {
                format!("Represent this sentence for searching relevant passages: {text}")
            }
            _ => text.clone(),
        })
        .collect();
    let key = match settings.auth_mode.as_str() {
        "api-key" => Some(
            key.filter(|key| !key.is_empty())
                .ok_or("recall_embedding_key_missing")?,
        ),
        "none" => None,
        _ => return Err("recall_embedding_auth_invalid".into()),
    };
    let response = crate::model_registry::transport::embeddings(
        base,
        key,
        serde_json::json!({"model":model,"input":input,"encoding_format":"float"}),
        cancelled,
    )
    .await
    .map_err(|code| crate::model_registry::transport::feature_error(&code, "recall_embedding"))?;
    decode(
        &serde_json::to_vec(&response).map_err(|_| "recall_embedding_response_invalid")?,
        texts.len(),
    )
}

fn decode(bytes: &[u8], count: usize) -> Result<Vec<Vec<f32>>, String> {
    let response: Response =
        serde_json::from_slice(bytes).map_err(|_| "recall_embedding_response_invalid")?;
    if response.data.len() != count {
        return Err("recall_embedding_output_count".into());
    }
    let mut ordered = vec![None; count];
    for row in response.data {
        if row.index >= count || ordered[row.index].is_some() {
            return Err("recall_embedding_output_order".into());
        }
        ordered[row.index] = Some(row.embedding);
    }
    ordered
        .into_iter()
        .map(|row| row.ok_or_else(|| "recall_embedding_output_order".into()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn restores_provider_order_and_rejects_duplicate_or_missing_rows() {
        assert_eq!(
            decode(
                br#"{"data":[{"index":1,"embedding":[2]},{"index":0,"embedding":[1]}]}"#,
                2
            )
            .unwrap(),
            vec![vec![1.0], vec![2.0]]
        );
        assert!(decode(
            br#"{"data":[{"index":0,"embedding":[2]},{"index":0,"embedding":[1]}]}"#,
            2
        )
        .is_err());
        assert!(decode(br#"{"data":[]}"#, 1).is_err());
    }
    #[test]
    fn precancelled_document_request_never_connects_or_uploads() {
        use std::net::TcpListener;
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let mut settings = crate::settings::RecallSettings::default().embedding;
        settings.provider = "openai-compatible".into();
        settings.enabled = true;
        settings.auth_mode = "none".into();
        settings.base_url = Some(format!("http://{}/v1", listener.local_addr().unwrap()));
        settings.model = Some("synthetic-model".into());
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                assert_eq!(
                    embed(
                        &settings,
                        None,
                        EmbeddingPurpose::Document,
                        &["synthetic private fixture".into()],
                        &AtomicBool::new(true)
                    )
                    .await
                    .unwrap_err(),
                    "recall_cancelled"
                );
            });
        assert_eq!(
            listener.accept().err().unwrap().kind(),
            std::io::ErrorKind::WouldBlock
        );
    }
    #[test]
    fn rate_limit_failure_can_be_retried_against_healthy_endpoint() {
        use std::io::{Read, Write};
        use std::net::TcpListener;
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let boundary = std::thread::spawn(move || {
            for rate_limited in [true, false] {
                let (mut socket, _) = listener.accept().unwrap();
                socket
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let _ = socket.read(&mut [0u8; 4096]).unwrap();
                if rate_limited {
                    socket.write_all(b"HTTP/1.1 429 Too Many Requests\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").unwrap();
                } else {
                    let body = r#"{"data":[{"index":0,"embedding":[1,2]}]}"#;
                    write!(
                        socket,
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    )
                    .unwrap();
                }
            }
        });
        let mut settings = crate::settings::RecallSettings::default().embedding;
        settings.provider = "openai-compatible".into();
        settings.enabled = true;
        settings.auth_mode = "none".into();
        settings.base_url = Some(format!("http://{address}/v1"));
        settings.model = Some("synthetic-model".into());
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let input = vec!["synthetic text".into()];
                let cancelled = AtomicBool::new(false);
                assert_eq!(
                    embed(
                        &settings,
                        None,
                        EmbeddingPurpose::Document,
                        &input,
                        &cancelled
                    )
                    .await
                    .unwrap_err(),
                    "recall_embedding_rate_limited"
                );
                assert_eq!(
                    embed(
                        &settings,
                        None,
                        EmbeddingPurpose::Document,
                        &input,
                        &cancelled
                    )
                    .await
                    .unwrap(),
                    vec![vec![1.0, 2.0]]
                );
            });
        boundary.join().unwrap();
    }

    #[test]
    fn anonymous_auth_sends_no_key_and_cancellation_drops_pending_call() {
        use std::io::{Read, Write};
        use std::net::TcpListener;
        use std::sync::Arc;
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let boundary = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut request = [0_u8; 4096];
            let size = socket.read(&mut request).unwrap();
            let request = std::str::from_utf8(&request[..size])
                .unwrap()
                .to_ascii_lowercase();
            assert!(!request.contains("authorization:"));
            assert!(!request.contains("synthetic-secret"));
            let body = r#"{"data":[{"index":0,"embedding":[1,2]}]}"#;
            write!(
                socket,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .unwrap();
            let (mut socket, _) = listener.accept().unwrap();
            let _ = socket.read(&mut [0_u8; 4096]);
            std::thread::sleep(Duration::from_millis(200));
        });
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let mut settings = crate::settings::RecallSettings::default().embedding;
                settings.provider = "openai-compatible".into();
                settings.enabled = true;
                settings.auth_mode = "none".into();
                settings.base_url = Some(format!("http://{address}/v1"));
                settings.model = Some("synthetic-model".into());
                assert_eq!(
                    embed(
                        &settings,
                        Some("synthetic-secret"),
                        EmbeddingPurpose::Query,
                        &["synthetic text".into()],
                        &AtomicBool::new(false)
                    )
                    .await
                    .unwrap(),
                    vec![vec![1.0, 2.0]]
                );
                let cancelled = Arc::new(AtomicBool::new(false));
                let signal = cancelled.clone();
                tokio::spawn(async move {
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    signal.store(true, Ordering::Release);
                });
                assert_eq!(
                    embed(
                        &settings,
                        None,
                        EmbeddingPurpose::Query,
                        &["synthetic text".into()],
                        &cancelled
                    )
                    .await
                    .unwrap_err(),
                    "recall_cancelled"
                );
            });
        boundary.join().unwrap();
    }
}
