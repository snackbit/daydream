use super::super::super::*;
use super::types::*;

pub(super) fn vision_provider_ready(app_settings: &settings::AppSettings) -> bool {
    app_settings.ai.vision.provider == "openai-compatible"
        && app_settings.ai.vision.base_url.is_some()
        && app_settings.ai.vision.api_key_ref.is_some()
        && app_settings.ai.vision.model.is_some()
        && app_settings.ai.ocr.enabled
}

pub(super) fn vision_request_config(
    storage: &AppStorage,
    app_settings: &settings::AppSettings,
) -> Result<VisionRequestConfig, AiError> {
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
    let api_key = secret_crypto::decrypt_sensitive_setting_with_platform_key(&envelope)
        .map_err(secret_error)?;

    Ok(VisionRequestConfig {
        api_key,
        base_url,
        model,
        provider: "openai-compatible".to_string(),
    })
}

pub(super) async fn request_vlm_observations(
    config: &VisionRequestConfig,
    frame: &ExtractedVisualFrame,
    ocr_regions: &[OcrCandidate],
) -> Result<Vec<PendingVlmObservation>, AiError> {
    let image_url = png_data_url(frame.path())?;
    let before_context = frame
        .before_context
        .as_ref()
        .map(|context| {
            Ok::<_, AiError>(serde_json::json!({
                "type": "image_url",
                "image_url": { "url": png_data_url(&context.path)? }
            }))
        })
        .transpose()?;
    let after_context = frame
        .after_context
        .as_ref()
        .map(|context| {
            Ok::<_, AiError>(serde_json::json!({
                "type": "image_url",
                "image_url": { "url": png_data_url(&context.path)? }
            }))
        })
        .transpose()?;
    let ocr_text = ocr_regions
        .iter()
        .map(|region| {
            format!(
                "[x={}, y={}, w={}, h={}, conf={}] {}",
                region.region.x,
                region.region.y,
                region.region.width,
                region.region.height,
                region
                    .confidence
                    .map(|value| format!("{value:.2}"))
                    .unwrap_or_else(|| "unknown".to_string()),
                region.text
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let mut user_content = vec![serde_json::json!({
        "type": "text",
        "text": format!(
            "Frame timestamp: {} ms\nSource frame: {}\nImage region in full-frame coordinates: x={}, y={}, width={}, height={}\nFull-frame dimensions: width={}, height={}\nFull-frame checkpoint: {}\nAbrupt transition: {}\nBefore context: {}\nAfter context: {}\nTesseract OCR candidates:\n{}",
            frame.frame_timestamp_ms,
            frame.source_frame_ref,
            frame.region.x,
            frame.region.y,
            frame.region.width,
            frame.region.height,
            frame.width,
            frame.height,
            frame.full_checkpoint,
            frame.abrupt_transition,
            frame
                .before_context
                .as_ref()
                .map(|context| format!("{} at {} ms", context.source_frame_ref, context.frame_timestamp_ms))
                .unwrap_or_else(|| "none".to_string()),
            frame
                .after_context
                .as_ref()
                .map(|context| format!("{} at {} ms", context.source_frame_ref, context.frame_timestamp_ms))
                .unwrap_or_else(|| "none".to_string()),
            if ocr_text.is_empty() { "(none)" } else { &ocr_text }
        )
    })];
    if let Some(context) = before_context {
        user_content.push(context);
    }
    user_content.push(serde_json::json!({
        "type": "image_url",
        "image_url": { "url": image_url }
    }));
    if let Some(context) = after_context {
        user_content.push(context);
    }
    let endpoint = vision_chat_completions_url(&config.base_url);
    let body = serde_json::json!({
        "model": config.model.as_str(),
        "response_format": { "type": "json_object" },
        "messages": [
            {
                "role": "system",
                "content": "Return only strict JSON. Schema: {\"observations\":[{\"kind\":\"corrected-ocr\"|\"screen-observation\",\"text\":\"...\",\"frameTimestampMs\":number,\"region\":{\"x\":number,\"y\":number,\"width\":number,\"height\":number},\"confidence\":number|null}]}. Use corrected-ocr for OCR corrections and screen-observation for visible activity or work context. Every item must use the provided primary frame timestamp and full-frame coordinates contained inside the provided image region."
            },
            {
                "role": "user",
                "content": user_content
            }
        ]
    });

    let response = reqwest::Client::new()
        .post(endpoint)
        .timeout(VLM_TIMEOUT)
        .header(AUTHORIZATION, format!("Bearer {}", config.api_key))
        .json(&body)
        .send()
        .await
        .map_err(|error| {
            if error.is_timeout() {
                ai_error("ai_vision_remote_timeout", "Vision provider timed out.")
            } else {
                ai_error(
                    "ai_vision_remote_request_failed",
                    "Vision provider request failed.",
                )
            }
        })?;
    let status = response.status();
    if !status.is_success() {
        return Err(AiError {
            code: "ai_vision_remote_status_failed".to_string(),
            message: format!("Vision provider failed with HTTP status {status}."),
        });
    }
    let response_body: VlmChatResponse = response.json().await.map_err(|_error| {
        ai_error(
            "ai_vision_remote_response_invalid",
            "Vision provider response was not valid JSON.",
        )
    })?;
    let content = response_body
        .choices
        .first()
        .and_then(|choice| choice.message.content.as_deref())
        .ok_or_else(|| {
            ai_error(
                "ai_vision_remote_response_invalid",
                "Vision provider response did not include message content.",
            )
        })?;
    let payload: VlmObservationPayload = serde_json::from_str(content).map_err(|_error| {
        ai_error(
            "ai_vision_observation_invalid",
            "Vision provider content did not match the required observation JSON schema.",
        )
    })?;
    validate_vlm_observations(payload, frame)
}

fn png_data_url(path: &Path) -> Result<String, AiError> {
    let image_bytes = fs::read(path).map_err(file_error)?;
    Ok(format!(
        "data:image/png;base64,{}",
        BASE64_STANDARD.encode(image_bytes)
    ))
}

fn vision_chat_completions_url(base_url: &str) -> String {
    let trimmed = base_url.trim_end_matches('/');
    if trimmed.ends_with("/chat/completions") {
        trimmed.to_string()
    } else if trimmed.ends_with("/v1") {
        format!("{trimmed}/chat/completions")
    } else {
        format!("{trimmed}/v1/chat/completions")
    }
}

pub(in crate::ai) fn validate_vlm_observations(
    payload: VlmObservationPayload,
    frame: &ExtractedVisualFrame,
) -> Result<Vec<PendingVlmObservation>, AiError> {
    let mut observations = Vec::new();
    for observation in payload.observations {
        if !matches!(
            observation.kind.as_str(),
            "corrected-ocr" | "screen-observation"
        ) {
            return Err(ai_error(
                "ai_vision_observation_invalid",
                "Vision observation kind is invalid.",
            ));
        }
        let text = observation.text.trim();
        if text.is_empty() {
            return Err(ai_error(
                "ai_vision_observation_invalid",
                "Vision observation text is empty.",
            ));
        }
        if observation.frame_timestamp_ms != frame.frame_timestamp_ms {
            return Err(ai_error(
                "ai_vision_observation_invalid",
                "Vision observation timestamp does not match the source frame.",
            ));
        }
        if observation.region.width <= 0
            || observation.region.height <= 0
            || observation.region.x < 0
            || observation.region.y < 0
            || observation.region.x >= frame.width
            || observation.region.y >= frame.height
            || observation.region.x < frame.region.x
            || observation.region.y < frame.region.y
            || observation.region.x + observation.region.width > frame.region.x + frame.region.width
            || observation.region.y + observation.region.height
                > frame.region.y + frame.region.height
        {
            return Err(ai_error(
                "ai_vision_observation_invalid",
                "Vision observation region is invalid.",
            ));
        }
        observations.push(PendingVlmObservation {
            confidence: observation.confidence.map(|value| value.clamp(0.0, 1.0)),
            frame_timestamp_ms: observation.frame_timestamp_ms,
            observation_kind: observation.kind,
            region: VisualRegion {
                height: observation.region.height.min(frame.height),
                width: observation.region.width.min(frame.width),
                x: observation.region.x,
                y: observation.region.y,
            },
            source_frame_ref: frame.source_frame_ref.clone(),
            text: text.to_string(),
        });
    }

    Ok(observations)
}
