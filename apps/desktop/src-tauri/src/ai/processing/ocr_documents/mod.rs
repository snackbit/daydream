use super::super::*;
use super::visual::VisionRequestConfig;
use crate::storage::{OcrContentInput, OcrFrameInput};
use std::collections::BTreeSet;

#[cfg(test)]
mod benchmark;
mod config;
pub(in crate::ai) mod extraction;
pub(in crate::ai) mod frame_cache;
mod recognition;
pub(in crate::ai) mod recognition_lease;
pub(in crate::ai) mod sampling;
use recognition::{raw_word_inputs, run_frame};
mod reprocess;
mod tesseract;
pub(in crate::ai) mod vision;
pub(crate) use config::capture_ocr_job_config;
use config::{load_chunk_ocr_config, OcrEngine};
use extraction::{extract_full_frames, FullFrame};
pub(crate) use reprocess::reprocess_recording_ocr;

pub(in crate::ai) async fn probe_provider(
    config: &VisionRequestConfig,
    path: &Path,
    cancel: Arc<AtomicBool>,
) -> Result<(), AiError> {
    let frame = FullFrame {
        path: path.into(),
        timestamp_ms: 0,
        end_ms: 1,
        width: 128,
        height: 64,
        pixel_hash: "synthetic-provider-test".into(),
        source_ref: "synthetic-provider-test".into(),
    };
    let recognized = vision::run_vision_document(config, &frame, cancel).await?;
    if recognized
        .text
        .split_whitespace()
        .collect::<String>()
        .eq_ignore_ascii_case("TEST")
    {
        Ok(())
    } else {
        Err(ai_error("ai_ocr_vision_test_text_mismatch","The provider could not read the generated test image. Review its image support and model."))
    }
}

mod chunk;
pub(in crate::ai) use chunk::process_ocr_document_chunk;

pub(in crate::ai) async fn infer_or_reuse_frame<F, Fut>(
    storage: &AppStorage,
    chunk: &ProcessingChunkRow,
    input: &mut OcrFrameInput,
    infer: F,
) -> Result<OcrContentInput, AiError>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<OcrContentInput, AiError>>,
{
    ensure_chunk_active(storage, chunk)?;
    let cached = storage
        .with_connection(|connection| {
            storage::load_cached_ocr_document(
                connection,
                &chunk.recording_id,
                &input.source_track_id,
                &input.pixel_hash,
                input.width,
                input.height,
                &input.config_fingerprint,
            )
        })
        .map_err(storage_error)?;
    if let Some(cached) = cached {
        input.cache_content_id = Some(cached.content_id);
        return Ok(cached.content);
    }
    let result = infer().await?;
    ensure_chunk_active(storage, chunk)?;
    Ok(result)
}
