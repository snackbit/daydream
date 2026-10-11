//! Only published, recognized provider codes can become diagnostic content.
use serde_json::Value;

const CODES: &[(&str, &str)] = &[
    (
        "subscription_sharing_usage_limit_exceeded",
        "model_subscription_usage_limit",
    ),
    (
        "subscription_sharing_usage_unavailable",
        "model_subscription_usage_unavailable",
    ),
    (
        "subscription_sharing_user_not_eligible",
        "model_subscription_ineligible",
    ),
    (
        "subscription_sharing_unsupported_capability",
        "model_capability_unsupported",
    ),
    (
        "subscription_sharing_route_not_supported",
        "model_subscription_route_unsupported",
    ),
    ("model_not_found", "model_not_found"),
    ("invalid_api_key", "model_auth_failed"),
    ("insufficient_quota", "model_rate_limited"),
    ("invalid_request_error", "model_request_invalid"),
    ("unsupported_parameter", "model_request_invalid"),
];
pub(super) fn known(value: &Value) -> Option<(&'static str, &'static str)> {
    let code = value["code"]
        .as_str()
        .or_else(|| value["error"]["code"].as_str())
        .or_else(|| value["response"]["error"]["code"].as_str())?;
    CODES.iter().copied().find(|(known, _)| *known == code)
}
pub(super) fn app_code(value: &Value) -> Option<&'static str> {
    known(value).map(|(_, code)| code)
}
