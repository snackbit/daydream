use super::super::super::*;
use super::types::*;

pub(super) fn vision_provider_ready(app_settings: &settings::AppSettings) -> bool {
    app_settings.ai.vision.observations_enabled && settings::vision_ready(&app_settings.ai.vision)
}

pub(in crate::ai) fn vision_request_config(
    storage: &AppStorage,
    app_settings: &settings::AppSettings,
) -> Result<VisionRequestConfig, AiError> {
    if app_settings
        .ai
        .vision
        .model_route
        .as_ref()
        .is_some_and(|route| {
            route.subscription
                && route
                    .subscription_id
                    .as_deref()
                    .is_none_or(|id| !crate::model_registry::subscription::configured(storage, id))
        })
    {
        return Err(ai_error(
            "ai_vision_remote_subscription_reconnect",
            "Sign in with ChatGPT again for the selected model.",
        ));
    }
    let base_url = app_settings
        .ai
        .vision
        .base_url
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            ai_error(
                "ai_vision_remote_unconfigured",
                "Vision base URL is not configured.",
            )
        })?
        .trim()
        .to_string();
    let model = app_settings
        .ai
        .vision
        .model
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            ai_error(
                "ai_vision_remote_unconfigured",
                "Vision model is not configured.",
            )
        })?
        .trim()
        .to_string();
    let api_key = if app_settings.ai.vision.auth_mode == "none"
        || app_settings
            .ai
            .vision
            .model_route
            .as_ref()
            .is_some_and(|route| route.subscription_id.is_some())
    {
        String::new()
    } else {
        let secret_ref = app_settings
            .ai
            .vision
            .api_key_ref
            .as_deref()
            .ok_or_else(|| {
                ai_error(
                    "ai_vision_remote_unconfigured",
                    "Vision API key is not configured.",
                )
            })?;
        let envelope = storage
            .with_connection(|connection| storage::get_app_setting(connection, secret_ref))
            .map_err(storage_error)?
            .ok_or_else(|| {
                ai_error(
                    "ai_vision_remote_unconfigured",
                    "Vision API key is not configured.",
                )
            })?
            .value_json;
        secret_crypto::decrypt_sensitive_setting_with_platform_key(&envelope).map_err(|_| {
            ai_error(
                "ai_vision_remote_credential_unavailable",
                "The saved vision credential could not be opened.",
            )
        })?
    };
    Ok(VisionRequestConfig {
        route: app_settings.ai.vision.model_route.clone(),
        storage: Some(storage.clone()),
        api_key,
        base_url,
        model,
        provider: "openai-compatible".to_string(),
    })
}

pub(in crate::ai) fn verify_current_vision_configuration(
    storage: &AppStorage,
    pinned: &VisionRequestConfig,
    ocr: bool,
) -> Result<(), AiError> {
    let current = settings::load_app_settings(storage).map_err(settings_error)?;
    if current.ai.vision.provider != "openai-compatible"
        || (ocr && (!current.ai.ocr.enabled || current.ai.ocr.engine != "vision"))
        || (!ocr && !current.ai.vision.observations_enabled)
    {
        return Err(ai_error(
            "ai_vision_configuration_changed",
            "Remote vision is no longer enabled for this source.",
        ));
    }
    let current = vision_request_config(storage, &current)?;
    if current.base_url != pinned.base_url
        || current.model != pinned.model
        || current.api_key != pinned.api_key
        || current.route != pinned.route
    {
        return Err(ai_error(
            "ai_vision_configuration_changed",
            "Vision configuration changed. Retry with current settings.",
        ));
    }
    Ok(())
}
