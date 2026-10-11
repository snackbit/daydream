use super::super::super::*;
use super::image::decode_png_rgba;
use super::types::*;

pub(super) use super::provider_configuration::vision_provider_ready;
pub(in crate::ai) use super::provider_configuration::{
    verify_current_vision_configuration, vision_request_config,
};

pub(super) async fn request_vlm_observations(
    config: &VisionRequestConfig,
    frame: &ExtractedVisualFrame,
    ocr_regions: &[OcrCandidate],
    previous_observations: &[PreviousVisualObservation],
    cancel: Arc<AtomicBool>,
) -> Result<Vec<PendingVlmObservation>, AiError> {
    let mut http_diagnostics = crate::provider_http::HttpDiagnostics::new("vision_observations");
    let result: Result<_, AiError> = async {
        let endpoint = vision_chat_completions_url(&config.base_url);
        let body = vlm_chat_completions_body(config, frame, ocr_regions, previous_observations)?;
        if config.route.is_some() {
            let response = crate::model_registry::transport::authenticated_completion(
                config.storage.as_ref(),
                &config.base_url,
                (!config.api_key.is_empty()).then_some(config.api_key.as_str()),
                config.route.as_ref(),
                body,
                1_000_000,
                &cancel,
            )
            .await
            .map_err(|code| {
                ai_error(
                    &crate::model_registry::transport::feature_error(&code, "ai_vision_remote"),
                    crate::model_registry::commands::friendly(&code),
                )
            })?;
            http_diagnostics.stage("output_validation");
            let response: VlmChatCompletionResponse =
                serde_json::from_value(response).map_err(|_| {
                    ai_error(
                        "ai_vision_remote_response_invalid",
                        "The provider returned an invalid response.",
                    )
                })?;
            return validate_vlm_observation_text(vlm_chat_completion_text(&response)?, frame);
        }
        let request_bytes = serde_json::to_vec(&body)
            .map(|bytes| bytes.len())
            .unwrap_or(0);

        tracing::info!(
            frame_timestamp_ms = frame.frame_timestamp_ms,
            source_frame_ref = %frame.source_frame_ref,
            provider = %config.provider,
            ocr_candidate_count = ocr_regions.len(),
            previous_observation_count = previous_observations.len(),
            request_bytes,
            "ai_vlm_request_started"
        );
        let started = Instant::now();
        let request = crate::provider_http::client()
            .post(endpoint)
            .timeout(VLM_TIMEOUT)
            .json(&body);
        let request = if config.api_key.is_empty() {
            request
        } else {
            request.header(AUTHORIZATION, format!("Bearer {}", config.api_key))
        };
        http_diagnostics.stage("http_request");
        let response =
            super::super::ocr_documents::vision::await_cancelled(request.send(), cancel.clone())
                .await?
                .map_err(|error| {
                    if error.is_timeout() {
                        tracing::warn!(
                            frame_timestamp_ms = frame.frame_timestamp_ms,
                            source_frame_ref = %frame.source_frame_ref,
                            elapsed_ms = elapsed_ms(started),
                            "ai_vlm_request_timeout"
                        );
                        ai_error("ai_vision_remote_timeout", "Vision provider timed out.")
                    } else {
                        tracing::warn!(
                            frame_timestamp_ms = frame.frame_timestamp_ms,
                            source_frame_ref = %frame.source_frame_ref,
                            elapsed_ms = elapsed_ms(started),
                            "ai_vlm_request_failed"
                        );
                        ai_error(
                            "ai_vision_remote_request_failed",
                            "Vision provider request failed.",
                        )
                    }
                })?;
        let status = response.status();
        http_diagnostics.response(
            status.as_u16(),
            response
                .headers()
                .get("x-request-id")
                .and_then(|v| v.to_str().ok()),
        );
        if !status.is_success() {
            tracing::warn!(
                frame_timestamp_ms = frame.frame_timestamp_ms,
                source_frame_ref = %frame.source_frame_ref,
                http_status = %status,
                elapsed_ms = elapsed_ms(started),
                "ai_vlm_response_status_failed"
            );
            return Err(AiError {
                code: format!(
                    "ai_vision_remote_{}",
                    crate::provider_http::status_code(status)
                ),
                message: format!("Vision provider failed with HTTP status {status}."),
            });
        }
        http_diagnostics.stage("http_json");
        let response_body: VlmChatCompletionResponse =
            super::super::ocr_documents::vision::await_cancelled(response.json(), cancel)
                .await?
                .map_err(|_error| {
                    tracing::warn!(
                        frame_timestamp_ms = frame.frame_timestamp_ms,
                        source_frame_ref = %frame.source_frame_ref,
                        http_status = %status,
                        elapsed_ms = elapsed_ms(started),
                        "ai_vlm_response_json_invalid"
                    );
                    ai_error(
                        "ai_vision_remote_response_invalid",
                        "Vision provider response was not valid JSON.",
                    )
                })?;
        http_diagnostics.stage("output_validation");
        let choice_count = response_body.choices.len();
        let content = match vlm_chat_completion_text(&response_body) {
            Ok(content) => content,
            Err(error) => {
                tracing::warn!(
                    frame_timestamp_ms = frame.frame_timestamp_ms,
                    source_frame_ref = %frame.source_frame_ref,
                    http_status = %status,
                    choice_count,
                    error_code = %error.code,
                    elapsed_ms = elapsed_ms(started),
                    "ai_vlm_response_text_missing"
                );
                return Err(error);
            }
        };
        let observations = match validate_vlm_observation_text(content, frame) {
            Ok(observations) => observations,
            Err(error) => {
                tracing::warn!(
                    frame_timestamp_ms = frame.frame_timestamp_ms,
                    source_frame_ref = %frame.source_frame_ref,
                    http_status = %status,
                    choice_count,
                    content_char_count = content.chars().count(),
                    error_code = %error.code,
                    elapsed_ms = elapsed_ms(started),
                    "ai_vlm_observation_payload_invalid"
                );
                return Err(error);
            }
        };
        tracing::info!(
            frame_timestamp_ms = frame.frame_timestamp_ms,
            source_frame_ref = %frame.source_frame_ref,
            http_status = %status,
            choice_count,
            observation_count = observations.len(),
            elapsed_ms = elapsed_ms(started),
            "ai_vlm_request_completed"
        );
        Ok(observations)
    }
    .await;
    http_diagnostics.finish(result.as_ref().err().map(|error| error.code.as_str()));
    result
}

pub(in crate::ai) fn vlm_chat_completions_body(
    config: &VisionRequestConfig,
    frame: &ExtractedVisualFrame,
    ocr_regions: &[OcrCandidate],
    previous_observations: &[PreviousVisualObservation],
) -> Result<serde_json::Value, AiError> {
    let image_url = vlm_png_data_url(frame.path())?;
    Ok(serde_json::json!({
        "model": config.model.as_str(),
        "messages": [
            {
                "role": "system",
                "content": VLM_DEVELOPER_PROMPT
            },
            {
                "role": "user",
                "content": [
                    {
                        "type": "text",
                        "text": vlm_user_prompt(frame, ocr_regions, previous_observations)
                    },
                    {
                        "type": "image_url",
                        "image_url": {
                            "url": image_url
                        }
                    }
                ]
            }
        ]
    }))
}

fn vlm_png_data_url(path: &Path) -> Result<String, AiError> {
    let image_bytes = vlm_png_bytes(path)?;
    Ok(format!(
        "data:image/png;base64,{}",
        BASE64_STANDARD.encode(image_bytes)
    ))
}

fn vlm_png_bytes(path: &Path) -> Result<Vec<u8>, AiError> {
    let image = match decode_png_rgba(path) {
        Ok(image) => image,
        Err(_error) => return fs::read(path).map_err(file_error),
    };
    let longest_edge = image.width.max(image.height);
    if longest_edge <= VLM_MAX_IMAGE_LONG_EDGE_PX {
        return fs::read(path).map_err(file_error);
    }
    let scale = VLM_MAX_IMAGE_LONG_EDGE_PX as f64 / longest_edge as f64;
    let scaled_width = ((image.width as f64 * scale).round() as usize).max(1);
    let scaled_height = ((image.height as f64 * scale).round() as usize).max(1);
    let mut scaled = Vec::with_capacity(scaled_width * scaled_height * 4);
    for y in 0..scaled_height {
        let source_y = ((y as f64 / scale).floor() as usize).min(image.height - 1);
        for x in 0..scaled_width {
            let source_x = ((x as f64 / scale).floor() as usize).min(image.width - 1);
            let source_index = ((source_y * image.width) + source_x) * 4;
            scaled.extend_from_slice(&image.pixels[source_index..source_index + 4]);
        }
    }

    encode_png_rgba(scaled_width, scaled_height, &scaled)
}

fn encode_png_rgba(width: usize, height: usize, pixels: &[u8]) -> Result<Vec<u8>, AiError> {
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, width as u32, height as u32);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().map_err(|_error| {
            ai_error(
                "ai_processing_visual_frame_invalid",
                "Visual frame PNG metadata could not be written.",
            )
        })?;
        writer.write_image_data(pixels).map_err(|_error| {
            ai_error(
                "ai_processing_visual_frame_invalid",
                "Visual frame PNG pixels could not be written.",
            )
        })?;
    }
    Ok(bytes)
}

pub(in crate::ai) fn vision_chat_completions_url(base_url: &str) -> String {
    let trimmed = base_url.trim_end_matches('/');
    if trimmed.ends_with("/chat/completions") {
        trimmed.to_string()
    } else if trimmed.ends_with("/v1") {
        format!("{trimmed}/chat/completions")
    } else {
        format!("{trimmed}/v1/chat/completions")
    }
}

fn vlm_user_prompt(
    frame: &ExtractedVisualFrame,
    ocr_regions: &[OcrCandidate],
    previous_observations: &[PreviousVisualObservation],
) -> String {
    format!(
        "Current frame metadata:\nSource frame: {}\nSource image dimensions: width={}, height={}\nSource region dimensions: width={}, height={}\nFull-frame checkpoint: {}\nAbrupt visual transition: {}\n\nPrevious screen descriptions from nearby earlier frames:\n{}\n\nTesseract OCR candidates for the current frame. These are hints only and may contain noise; do not copy isolated symbols, punctuation, or single-character candidates into the response:\n{}",
        frame.source_frame_ref,
        frame.width,
        frame.height,
        frame.region.width,
        frame.region.height,
        frame.full_checkpoint,
        frame.abrupt_transition,
        previous_observations_text(previous_observations),
        ocr_candidates_text(ocr_regions),
    )
}

fn previous_observations_text(previous_observations: &[PreviousVisualObservation]) -> String {
    if previous_observations.is_empty() {
        return "(none)".to_string();
    }

    previous_observations
        .iter()
        .map(|observation| format!("- {}: {}", observation.kind, observation.text))
        .collect::<Vec<_>>()
        .join("\n")
}

fn ocr_candidates_text(ocr_regions: &[OcrCandidate]) -> String {
    if ocr_regions.is_empty() {
        return "(none)".to_string();
    }

    let mut text = String::new();
    let mut included_count = 0usize;
    for line in ocr_regions
        .iter()
        .filter(|region| !region.text.trim().is_empty())
        .take(VLM_MAX_OCR_CANDIDATES)
        .map(|region| {
            format!(
                "- conf={}: {}",
                region
                    .confidence
                    .map(|value| format!("{value:.2}"))
                    .unwrap_or_else(|| "unknown".to_string()),
                region.text
            )
        })
    {
        let line_char_count = line.chars().count();
        let separator_chars = usize::from(!text.is_empty());
        if text.chars().count() + separator_chars + line_char_count > VLM_MAX_OCR_TEXT_CHARS {
            break;
        }
        if !text.is_empty() {
            text.push('\n');
        }
        text.push_str(&line);
        included_count += 1;
    }
    if text.is_empty() {
        return "(none)".to_string();
    }
    if ocr_regions.len() > included_count {
        text.push_str(&format!(
            "\n- ({}) additional OCR candidates omitted from the prompt budget",
            ocr_regions.len() - included_count
        ));
    }
    text
}

const VLM_DEVELOPER_PROMPT: &str = "Daydream is a local-first desktop recall assistant. You are analyzing sampled frames from a private screen recording. Return plain text only: no JSON, no Markdown, no bullet list, no headings. Describe what is visibly happening on the screen accurately and comprehensively in multiple paragraphs. At most 1000 words. Prioritize visible app and window layout, active content, UI state, media, chat, document, browser, terminal, or tool context, and meaningful changes from previous frames. Additional context provided may be OCR provided by tesseract which may be inaccurate but good for starters. Treat those candidates as hints only; do not necessarily perform OCR. Do not mention coordinates, timestamps, hidden user intent, or unverifiable claims. Main purpose of this VLM pass is summarizing \"what happened\" and indexing for proper retrieval.";

fn vlm_chat_completion_text(response: &VlmChatCompletionResponse) -> Result<&str, AiError> {
    response
        .choices
        .iter()
        .find_map(|choice| choice.message.content.as_deref())
        .map(str::trim)
        .filter(|content| !content.is_empty())
        .ok_or_else(|| {
            ai_error(
                "ai_vision_remote_response_invalid",
                "Vision provider response did not include output text.",
            )
        })
}

pub(in crate::ai) fn validate_vlm_observation_text(
    content: &str,
    frame: &ExtractedVisualFrame,
) -> Result<Vec<PendingVlmObservation>, AiError> {
    let text = content.trim();
    if text.is_empty() {
        return Err(ai_error(
            "ai_vision_observation_invalid",
            "Vision provider returned an empty screen description.",
        ));
    }

    Ok(vec![PendingVlmObservation {
        confidence: None,
        frame_timestamp_ms: frame.frame_timestamp_ms,
        observation_kind: "screen-observation".to_string(),
        region: VisualRegion {
            height: frame.region.height,
            width: frame.region.width,
            x: frame.region.x,
            y: frame.region.y,
        },
        source_frame_ref: frame.source_frame_ref.clone(),
        text: text.to_string(),
    }])
}
