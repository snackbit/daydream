//! SSE framing and completed text ownership, shared by subscription consumers.
use super::{diagnostics, subscription_failure};
use futures_util::StreamExt;
use serde_json::{json, Value};
use std::collections::BTreeMap;

/// Validate whole events before the SDK can log malformed JSON. Re-encode to
/// one data line, so framing, keepalives and multiline data have one owner.
pub(super) async fn normalize_sse(bytes: Vec<u8>) -> Result<Vec<u8>, String> {
    diagnostics::stage("sse_decode");
    // The parser slices a leading BOM at byte 1 and panics; strip the marker
    // without cloning before handing the input to eventsource-stream 0.2.3.
    let data = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(&bytes);
    let chunks = futures_util::stream::iter([Ok::<_, std::io::Error>(data)]);
    let mut events = eventsource_stream::EventStream::new(chunks);
    let mut output = Vec::new();
    let mut count = 0;
    let mut keepalives = 0;
    while let Some(event) = events.next().await {
        let event = event.map_err(|_| {
            diagnostics::failure(
                "sse_decode",
                "invalid_framing_or_utf8",
                "model_response_invalid",
            )
        })?;
        if event.event == "keepalive" {
            keepalives += 1;
            continue;
        }
        if event.data.trim() == "[DONE]" {
            output.extend_from_slice(b"data: [DONE]\n\n");
            break;
        }
        let value: Value = serde_json::from_str(&event.data)
            .map_err(|error| diagnostics::json_failure("sse_json", &error))?;
        count += 1;
        output.extend_from_slice(b"data: ");
        serde_json::to_writer(&mut output, &value)
            .map_err(|error| diagnostics::json_failure("sse_json", &error))?;
        output.extend_from_slice(b"\n\n");
    }
    tracing::info!(
        event_count = count,
        keepalive_count = keepalives,
        "model_stream_decoded"
    );
    Ok(output)
}

#[derive(Default)]
pub(super) struct StreamOutput {
    parts: BTreeMap<(u64, u64), String>,
    bytes: usize,
    completed: Option<Value>,
    events: usize,
}
impl StreamOutput {
    pub(super) fn accept(&mut self, event: Value, limit: usize) -> Result<(), String> {
        self.events += 1;
        match event["type"].as_str() {
            Some("response.output_text.delta" | "response.output_text.done") => {
                if self.completed.is_some() {
                    return Err(diagnostics::failure(
                        "stream_output",
                        "text_after_completion",
                        "model_response_invalid",
                    ));
                }
                let index = (
                    event["output_index"].as_u64().ok_or_else(|| {
                        diagnostics::failure(
                            "stream_output",
                            "missing_output_index",
                            "model_response_invalid",
                        )
                    })?,
                    event["content_index"].as_u64().ok_or_else(|| {
                        diagnostics::failure(
                            "stream_output",
                            "missing_content_index",
                            "model_response_invalid",
                        )
                    })?,
                );
                let done = event["type"] == "response.output_text.done";
                let text = event[if done { "text" } else { "delta" }]
                    .as_str()
                    .ok_or_else(|| {
                        diagnostics::failure(
                            "stream_output",
                            "invalid_text_part",
                            "model_response_invalid",
                        )
                    })?;
                let part = self.parts.entry(index).or_default();
                self.bytes = if done {
                    self.bytes
                        .saturating_sub(part.len())
                        .saturating_add(text.len())
                } else {
                    self.bytes.saturating_add(text.len())
                };
                if self.bytes > limit {
                    return Err(diagnostics::failure(
                        "stream_output",
                        "text_limit",
                        "model_response_limit",
                    ));
                }
                if done {
                    *part = text.into();
                } else {
                    part.push_str(text);
                }
            }
            Some("response.completed") => {
                if self.completed.is_some() {
                    return Err(diagnostics::failure(
                        "stream_output",
                        "duplicate_completion",
                        "model_response_invalid",
                    ));
                }
                self.completed = Some(event["response"].clone());
            }
            Some("response.failed" | "response.incomplete" | "error") => {
                diagnostics::provider_failure(&event);
                tracing::warn!(
                    event_count = self.events,
                    text_part_count = self.parts.len(),
                    output_bytes = self.bytes,
                    "model_stream_failed"
                );
                let code = subscription_failure(&event).unwrap_or(
                    if event["type"] == "response.incomplete" {
                        "model_response_incomplete"
                    } else {
                        "model_subscription_request_failed"
                    },
                );
                return Err(diagnostics::failure(
                    "stream_output",
                    "terminal_failure",
                    code,
                ));
            }
            _ => {}
        }
        Ok(())
    }
    pub(super) fn finish(self, limit: usize) -> Result<String, String> {
        tracing::info!(
            event_count = self.events,
            text_part_count = self.parts.len(),
            output_bytes = self.bytes,
            completed = self.completed.is_some(),
            "model_stream_finished"
        );
        let response = self.completed.ok_or_else(|| {
            diagnostics::failure(
                "stream_output",
                "missing_completion",
                "model_response_incomplete",
            )
        })?;
        let text = completed_text(&response, limit, true)?;
        if !text.is_empty() {
            return Ok(text);
        }
        let text: String = self.parts.into_values().collect();
        if text.is_empty() {
            return Err(diagnostics::failure(
                "completion_output",
                "empty_text",
                "model_response_invalid",
            ));
        }
        Ok(text)
    }
}
pub(super) fn completed_text(
    response: &Value,
    limit: usize,
    allow_empty: bool,
) -> Result<String, String> {
    diagnostics::stage("completion_output");
    if response["status"] != "completed" {
        let status = match response["status"].as_str() {
            Some("incomplete") => "incomplete",
            Some("failed") => "failed",
            Some("cancelled") => "cancelled",
            Some("queued") => "queued",
            Some("in_progress") => "in_progress",
            _ => "unrecognized",
        };
        tracing::warn!(completion_status = status, "model_completion_not_completed");
        return Err(diagnostics::failure(
            "completion_output",
            "not_completed",
            "model_response_incomplete",
        ));
    }
    let output = response["output"].as_array().ok_or_else(|| {
        diagnostics::failure(
            "completion_output",
            "missing_output_array",
            "model_response_invalid",
        )
    })?;
    let mut text = String::new();
    for item in output {
        if item["type"] != "message" {
            continue;
        }
        for content in item["content"].as_array().ok_or_else(|| {
            diagnostics::failure(
                "completion_output",
                "missing_content_array",
                "model_response_invalid",
            )
        })? {
            if content["type"] == "refusal" {
                return Err(diagnostics::failure(
                    "completion_output",
                    "refusal",
                    "model_response_invalid",
                ));
            }
            if content["type"] == "output_text" {
                text.push_str(content["text"].as_str().ok_or_else(|| {
                    diagnostics::failure(
                        "completion_output",
                        "invalid_text",
                        "model_response_invalid",
                    )
                })?);
                if text.len() > limit {
                    return Err(diagnostics::failure(
                        "completion_output",
                        "text_limit",
                        "model_response_limit",
                    ));
                }
            }
        }
    }
    tracing::debug!(
        output_count = output.len(),
        output_bytes = text.len(),
        "model_completion_decoded"
    );
    if text.is_empty() && !allow_empty {
        return Err(diagnostics::failure(
            "completion_output",
            "empty_text",
            "model_response_invalid",
        ));
    }
    Ok(text)
}
pub(super) fn envelope(text: String) -> Value {
    json!({"choices":[{"finish_reason":"stop","message":{"content":text}}]})
}
