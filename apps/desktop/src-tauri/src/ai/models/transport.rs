use super::super::*;

pub(super) async fn send_request(
    request: reqwest::RequestBuilder,
    manager: &AiModelManager,
    model_id: &str,
) -> Result<Option<reqwest::Response>, AiError> {
    let mut diagnostics = crate::provider_http::HttpDiagnostics::new("model_download");
    diagnostics.stage("http_request");
    let result = tokio::select! { result=request.send()=>result.map(Some).map_err(download_error), _=manager.stopped(model_id)=>Ok(None) };
    if let Ok(Some(response)) = &result {
        diagnostics.response(
            response.status().as_u16(),
            response
                .headers()
                .get("x-request-id")
                .and_then(|v| v.to_str().ok()),
        );
    }
    diagnostics.finish(result.as_ref().err().map(|error| error.code.as_str()));
    result
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum StreamRead<T> {
    Chunk(T),
    Stopped,
    Stalled,
}

pub(super) async fn read_chunk<S: futures_util::Stream + Unpin>(
    stream: &mut S,
    manager: &AiModelManager,
    model_id: &str,
) -> StreamRead<Option<S::Item>> {
    tokio::select! { chunk=stream.next()=>StreamRead::Chunk(chunk), _=manager.stopped(model_id)=>StreamRead::Stopped, _=tokio::time::sleep(Duration::from_secs(3))=>StreamRead::Stalled }
}

pub(in crate::ai) fn validate_content_range(
    value: Option<&str>,
    offset: u64,
    pinned_size: i64,
) -> Result<(), AiError> {
    let valid = value
        .and_then(|value| value.strip_prefix("bytes "))
        .and_then(|value| value.split_once('/'))
        .and_then(|(range, total)| {
            let (start, end) = range.split_once('-')?;
            Some((
                start.parse::<u64>().ok()?,
                end.parse::<u64>().ok()?,
                total.parse::<u64>().ok()?,
            ))
        })
        .is_some_and(|(start, end, total)| {
            start == offset && end >= start && end < total && total == pinned_size as u64
        });
    if valid {
        Ok(())
    } else {
        Err(AiError {
            code: "ai_model_download_range_invalid".into(),
            message:
                "The server returned inconsistent partial data. Remove partial data and retry."
                    .into(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn partial_response_must_match_actual_offset_and_pinned_total() {
        assert!(validate_content_range(Some("bytes 3-8/9"), 3, 9).is_ok());
        for value in [
            None,
            Some("bytes 0-8/9"),
            Some("bytes 3-8/10"),
            Some("bytes 3-9/9"),
            Some("bytes 3-8/*"),
        ] {
            assert!(validate_content_range(value, 3, 9).is_err());
        }
    }
    #[tokio::test]
    async fn pause_interrupts_a_stalled_http_body() {
        use std::net::TcpListener;
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let (release, wait) = std::sync::mpsc::channel::<()>();
        let server = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut request = Vec::new();
            loop {
                let mut chunk = [0; 1024];
                let length = socket.read(&mut chunk).unwrap();
                assert!(
                    length > 0,
                    "Synthetic request closed before its headers were complete"
                );
                request.extend_from_slice(&chunk[..length]);
                assert!(
                    request.len() <= 8192,
                    "Synthetic request headers exceeded the fixture limit"
                );
                if request.windows(4).any(|window| window == b"\r\n\r\n") {
                    break;
                }
            }
            assert!(request.starts_with(b"GET /synthetic HTTP/1.1\r\n"));
            socket
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 9\r\n\r\nabc")
                .unwrap();
            wait.recv().unwrap();
        });
        let manager = AiModelManager::new();
        manager.request_download("synthetic");
        let response = send_request(
            reqwest::Client::new().get(format!("http://{address}/synthetic")),
            &manager,
            "synthetic",
        )
        .await
        .unwrap()
        .unwrap();
        let mut stream = response.bytes_stream();
        assert!(matches!(
            read_chunk(&mut stream, &manager, "synthetic").await,
            StreamRead::Chunk(Some(Ok(_)))
        ));
        manager.pause_download("synthetic");
        assert!(matches!(
            tokio::time::timeout(
                Duration::from_secs(1),
                read_chunk(&mut stream, &manager, "synthetic")
            )
            .await
            .unwrap(),
            StreamRead::Stopped
        ));
        release.send(()).unwrap();
        server.join().unwrap();
    }
}
