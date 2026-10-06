pub(super) fn automatic_retry_limit(code: &str) -> i64 {
    if code.ends_with("response_invalid") || code.ends_with("schema_invalid") {
        return 1;
    }
    if code.ends_with("timeout")
        || code.ends_with("request_failed")
        || code.ends_with("rate_limited")
        || code == "ai_transcription_local_worker_crashed"
        || code.ends_with("server_failed")
    {
        return 2;
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn configuration_and_broken_bundles_never_retry_automatically() {
        for code in [
            "ai_transcription_vad_model_missing",
            "ai_ocr_vision_auth_failed",
            "ai_vision_remote_unconfigured",
        ] {
            assert_eq!(automatic_retry_limit(code), 0);
        }
        assert_eq!(
            automatic_retry_limit("ai_transcription_local_worker_crashed"),
            2
        );
        assert_eq!(automatic_retry_limit("ai_ocr_vision_response_invalid"), 1);
    }
}
