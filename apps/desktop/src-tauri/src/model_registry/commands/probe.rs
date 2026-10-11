//! Synthetic capability probes; never read captured media here.
use super::super::{
    transport,
    types::{Capability, Connection, Model, Route},
};
use super::key;
use crate::storage::AppStorage;
use serde_json::json;
use std::sync::atomic::AtomicBool;

pub(super) async fn probe(
    storage: &AppStorage,
    connection: &Connection,
    model: &Model,
    capability: &Capability,
    draft: Option<&str>,
    cancelled: &AtomicBool,
) -> Result<Option<u32>, String> {
    let key = key(storage, connection, draft)?;
    if *capability == Capability::Embedding {
        let response=transport::embeddings(&connection.base_url,key.as_deref(),json!({"model":model.model,"input":["Synthetic model connection test"],"encoding_format":"float"}),cancelled).await?;
        let data = response["data"]
            .as_array()
            .filter(|data| data.len() == 1)
            .ok_or("model_response_invalid")?;
        let vector = data[0]["embedding"]
            .as_array()
            .filter(|v| {
                !v.is_empty()
                    && v.len() <= 8192
                    && v.iter().all(|n| n.as_f64().is_some_and(f64::is_finite))
            })
            .ok_or("model_response_invalid")?;
        return Ok(Some(vector.len() as u32));
    }
    let content = if *capability == Capability::Vision {
        json!([{"type":"text","text":"Return JSON only: {\"color\":\"red\"} if the image is solid red. Otherwise report its color."},{"type":"image_url","image_url":{"url":synthetic_image()?}}])
    } else {
        json!("Return exactly the JSON object {\"ok\":true}.")
    };
    let body = json!({"model":model.model,"max_tokens":1024,"response_format":{"type":"json_object"},"messages":[{"role":"user","content":content}]});
    let response = transport::authenticated_completion(
        Some(storage),
        &connection.base_url,
        key.as_deref(),
        Some(&Route::from(connection)),
        body,
        1024 * 1024,
        cancelled,
    )
    .await?;
    tracing::Span::current().record("stage", "capability_completion");
    if response["choices"][0]["finish_reason"] != "stop" {
        let reason = match response["choices"][0]["finish_reason"].as_str() {
            Some("length") => "length",
            Some("content_filter") => "content_filter",
            Some("tool_calls") => "tool_calls",
            Some("function_call") => "function_call",
            _ => "unrecognized",
        };
        tracing::warn!(
            finish_reason = reason,
            stage = "capability_completion",
            "model_capability_incomplete"
        );
        return Err("model_response_incomplete".into());
    }
    let text = response["choices"][0]["message"]["content"]
        .as_str()
        .ok_or("model_response_invalid")?;
    tracing::Span::current().record("stage", "capability_json");
    let output: serde_json::Value = serde_json::from_str(text).map_err(|error| {
        tracing::warn!(stage = "capability_json", category = ?error.classify(), line = error.line(), column = error.column(), output_bytes = text.len(), "model_capability_json_invalid");
        "model_response_invalid"
    })?;
    if (*capability == Capability::Text && output["ok"] != true)
        || (*capability == Capability::Vision && output["color"] != "red")
    {
        tracing::Span::current().record("stage", "capability_validation");
        tracing::warn!(
            stage = "capability_validation",
            reason = "unexpected_synthetic_answer",
            "model_capability_validation_failed"
        );
        return Err("model_response_invalid".into());
    }
    Ok(None)
}
fn synthetic_image() -> Result<String, String> {
    use base64::Engine;
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, 32, 16);
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder
            .write_header()
            .map_err(|_| "model_test_image_failed")?;
        writer
            .write_image_data(&[255, 0, 0].repeat(32 * 16))
            .map_err(|_| "model_test_image_failed")?;
    }
    Ok(format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    ))
}
