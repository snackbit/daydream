//! Loopback callback admission. Unrelated browser requests cannot end sign-in.
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

fn parse(request: &[u8], state: &str) -> Result<Option<(String, Option<String>)>, &'static str> {
    let request =
        std::str::from_utf8(request).map_err(|_| "model_subscription_callback_invalid")?;
    let target = request
        .lines()
        .next()
        .and_then(|line| line.strip_prefix("GET "))
        .and_then(|line| line.split_whitespace().next())
        .ok_or("model_subscription_callback_invalid")?;
    let url = reqwest::Url::parse(&format!("http://127.0.0.1{target}"))
        .map_err(|_| "model_subscription_callback_invalid")?;
    if url.path() != "/auth/callback" {
        return Ok(None);
    }
    let pairs = url.query_pairs().collect::<Vec<_>>();
    for name in ["state", "code", "client_id", "error"] {
        if pairs.iter().filter(|(key, _)| key == name).count() > 1 {
            return Err("model_subscription_callback_invalid");
        }
    }
    let get = |name: &str| {
        pairs
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.to_string())
    };
    if get("state").as_deref() != Some(state) {
        return Ok(None);
    }
    if get("error").is_some() {
        return Err("model_subscription_sign_in_denied");
    }
    let code = get("code")
        .filter(|code| !code.is_empty())
        .ok_or("model_subscription_callback_invalid")?;
    // OpenAI returns `client_id` on initial dynamic registration; returning
    // authorization may omit it and retains the selected registration's ID.
    let issued = get("client_id");
    if issued.as_ref().is_some_and(|id| {
        id.is_empty()
            || id == "dynamic_agent_client"
            || id.len() > 256
            || id.chars().any(char::is_control)
    }) {
        return Err("model_subscription_callback_invalid");
    }
    Ok(Some((code, issued)))
}

pub(super) async fn callback(
    listener: &tokio::net::TcpListener,
    state: &str,
    cancelled: &Arc<AtomicBool>,
) -> Result<(String, Option<String>), String> {
    loop {
        if cancelled.load(Ordering::Acquire) {
            return Err("model_cancelled".into());
        }
        let (mut socket, _) = listener
            .accept()
            .await
            .map_err(|_| "model_subscription_callback_failed")?;
        let mut bytes = Vec::new();
        let mut buffer = [0; 1024];
        let read = async {
            loop {
                let count = socket
                    .read(&mut buffer)
                    .await
                    .map_err(|_| "model_subscription_callback_failed")?;
                if count == 0 {
                    return Err("model_subscription_callback_invalid");
                }
                if bytes.len() + count > 16384 {
                    return Err("model_subscription_callback_invalid");
                }
                bytes.extend_from_slice(&buffer[..count]);
                if bytes.windows(4).any(|v| v == b"\r\n\r\n") {
                    return Ok(());
                }
            }
        };
        match tokio::time::timeout(Duration::from_secs(2), read).await {
            Ok(Ok(())) => {}
            Ok(Err(code)) => {
                tracing::warn!(error_code = code, "chatgpt_callback_read_rejected");
                continue;
            }
            Err(_) => {
                tracing::warn!("chatgpt_callback_read_timeout");
                continue;
            }
        }
        let result = parse(&bytes, state);
        let accepted = matches!(result, Ok(Some(_)));
        let body = match &result {
            Ok(Some(_)) => "Authorization received. Daydream is finishing sign-in. Return to Daydream to check the result.",
            Err("model_subscription_sign_in_denied") => "Sign-in was declined. Return to Daydream to try again.",
            _ => "This sign-in request was not accepted.",
        };
        let response = format!("HTTP/1.1 {}\r\nContent-Type: text/plain\r\nCache-Control: no-store\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", if accepted { "200 OK" } else { "400 Bad Request" }, body.len());
        let _ = tokio::time::timeout(
            Duration::from_secs(2),
            socket.write_all(response.as_bytes()),
        )
        .await;
        match result {
            Ok(Some((code, issued))) => {
                tracing::info!(
                    has_client_id = issued.is_some(),
                    "chatgpt_callback_accepted"
                );
                return Ok((code, issued));
            }
            Err("model_subscription_sign_in_denied") => {
                tracing::warn!("chatgpt_callback_denied");
                return Err("model_subscription_sign_in_denied".into());
            }
            Err(code) => tracing::warn!(error_code = code, "chatgpt_callback_rejected"),
            Ok(None) => tracing::warn!(reason = "path_or_state", "chatgpt_callback_rejected"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn send(address: std::net::SocketAddr, target: &str) -> String {
        let mut browser = tokio::net::TcpStream::connect(address).await.unwrap();
        browser
            .write_all(format!("GET {target} HTTP/1.1\r\nHost: localhost\r\n\r\n").as_bytes())
            .await
            .unwrap();
        let mut response = String::new();
        browser.read_to_string(&mut response).await.unwrap();
        response
    }
    async fn send_callback(target: &str) -> (Result<(String, Option<String>), String>, String) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let cancelled = Arc::new(AtomicBool::new(false));
        let receive =
            tokio::spawn(async move { callback(&listener, "synthetic-state", &cancelled).await });
        let response = send(address, target).await;
        let result = tokio::time::timeout(Duration::from_secs(3), receive)
            .await
            .unwrap()
            .unwrap();
        (result, response)
    }
    #[tokio::test]
    async fn new_registration_reads_the_documented_client_id() {
        let (result, response) = send_callback(
            "/auth/callback?state=synthetic-state&code=synthetic-code&client_id=oaiapp_synthetic",
        )
        .await;
        assert_eq!(
            result.unwrap(),
            ("synthetic-code".into(), Some("oaiapp_synthetic".into()))
        );
        assert!(response.contains("finishing sign-in"));
        assert!(!response.contains("synthetic-code"));
    }
    #[tokio::test]
    async fn denied_sign_in_does_not_tell_the_browser_it_was_received_successfully() {
        let (result, response) =
            send_callback("/auth/callback?state=synthetic-state&error=access_denied").await;
        assert_eq!(result.unwrap_err(), "model_subscription_sign_in_denied");
        assert!(response.starts_with("HTTP/1.1 400"));
        assert!(!response.contains("Sign-in received"));
    }
    #[tokio::test]
    async fn unrelated_and_malformed_requests_do_not_consume_the_pending_sign_in() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let receive = tokio::spawn(async move {
            callback(
                &listener,
                "synthetic-state",
                &Arc::new(AtomicBool::new(false)),
            )
            .await
        });
        for target in [
            "/favicon.ico",
            "/auth/callback?state=wrong&code=synthetic-code",
            "/auth/callback?state=synthetic-state&code=one&code=two",
            "/auth/callback?state=synthetic-state&code=synthetic-code&client_id=one&client_id=two",
        ] {
            let response = send(address, target).await;
            assert!(response.starts_with("HTTP/1.1 400"));
            assert!(!response.contains("synthetic-code"));
        }
        send(
            address,
            "/auth/callback?state=synthetic-state&code=synthetic-returning-code",
        )
        .await;
        let result = tokio::time::timeout(Duration::from_secs(3), receive)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(result, ("synthetic-returning-code".into(), None));
    }
}
