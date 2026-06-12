use super::super::*;

pub(in crate::ai) fn vlm_state_for_recording(
    app_settings: &settings::AppSettings,
    chunks: &[ProcessingChunkRow],
    observation_count: usize,
) -> VlmState {
    let provider_status = vlm_provider_status(app_settings);
    if provider_status == "disabled" {
        return VlmState {
            error_code: None,
            provider_status,
            status: "disabled".to_string(),
        };
    }
    if provider_status != "configured" {
        return VlmState {
            error_code: None,
            provider_status,
            status: "unconfigured".to_string(),
        };
    }
    if chunks.is_empty() {
        return VlmState {
            error_code: None,
            provider_status,
            status: "not-planned".to_string(),
        };
    }
    if chunks.iter().any(|chunk| chunk.status == "running") {
        return VlmState {
            error_code: None,
            provider_status,
            status: "running".to_string(),
        };
    }
    if chunks.iter().any(|chunk| chunk.status == "queued") {
        return VlmState {
            error_code: None,
            provider_status,
            status: "queued".to_string(),
        };
    }
    if let Some(chunk) = chunks
        .iter()
        .find(|chunk| matches!(chunk.status.as_str(), "failed" | "cancelled"))
    {
        return VlmState {
            error_code: chunk
                .error_code
                .clone()
                .or_else(|| chunk.result_code.clone()),
            provider_status,
            status: "failed".to_string(),
        };
    }
    let status = if observation_count > 0 {
        "succeeded"
    } else if chunks.iter().all(|chunk| chunk.status == "succeeded") {
        "empty"
    } else {
        "not-planned"
    };

    VlmState {
        error_code: None,
        provider_status,
        status: status.to_string(),
    }
}

fn vlm_provider_status(app_settings: &settings::AppSettings) -> String {
    if app_settings.ai.vision.provider == "disabled" {
        "disabled".to_string()
    } else if app_settings.ai.vision.base_url.is_none() {
        "missing-base-url".to_string()
    } else if app_settings.ai.vision.api_key_ref.is_none() {
        "missing-key".to_string()
    } else if app_settings.ai.vision.model.is_none() {
        "missing-model".to_string()
    } else {
        "configured".to_string()
    }
}
