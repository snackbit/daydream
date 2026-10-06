use std::sync::OnceLock;
pub(crate) fn client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(reqwest::Client::new)
}
pub(crate) fn status_code(status: reqwest::StatusCode) -> &'static str {
    match status.as_u16() {
        401 | 403 => "auth_failed",
        404 => "model_not_found",
        408 | 504 => "timeout",
        429 => "rate_limited",
        500..=599 => "server_failed",
        400 | 413 | 415 | 422 => "request_invalid",
        _ => "status_failed",
    }
}
