//! Bounded shared extraction. Owners keep temporary images alive until use ends.
use super::*;
use extraction::FullFrameSet;
use sha2::{Digest, Sha256};
use std::collections::VecDeque;

const MAX_BYTES: u64 = 256 * 1024 * 1024;
const MAX_ENTRIES: usize = 4;
type Cache = VecDeque<(String, String, Arc<FullFrameSet>, u64)>;
fn cache() -> &'static Mutex<Cache> {
    static CACHE: OnceLock<Mutex<Cache>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(VecDeque::new()))
}
pub(in crate::ai) fn discard(jobs: &[String]) {
    cache()
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .retain(|entry| !jobs.contains(&entry.1));
    sampling::discard(jobs);
}

pub(in crate::ai) async fn extract(
    app: &AppHandle,
    chunk: &ProcessingChunkRow,
    segment: RecordingSegmentRow,
    offset: i64,
    fps: f64,
    cancel: Arc<AtomicBool>,
    lease: crate::recording_privacy::ProcessingLease,
) -> Result<Arc<FullFrameSet>, AiError> {
    let storage = app.state::<AppStorage>();
    ensure_chunk_active(&storage, chunk)?;
    let cuts = storage
        .with_connection(|connection| storage::list_redactions(connection, &chunk.recording_id))
        .map_err(storage_error)?;
    let metadata = fs::metadata(&segment.file_path).map_err(file_error)?;
    let key = format!(
        "{:x}",
        Sha256::digest(
            serde_json::to_vec(&(
                &chunk.job_id,
                &chunk.track_id,
                &segment.id,
                chunk.start_offset_ms,
                chunk.end_offset_ms,
                fps,
                metadata.len(),
                metadata.modified().ok(),
                cuts.iter()
                    .map(|cut| (
                        &cut.id,
                        cut.start_offset_ms,
                        cut.end_offset_ms,
                        &cut.track_ids_json
                    ))
                    .collect::<Vec<_>>(),
            ))
            .map_err(|_| ai_error(
                "ai_processing_config_invalid",
                "Frame configuration is invalid."
            ))?
        )
    );
    let _owner = recognition_lease::acquire(
        &chunk.recording_id,
        chunk.track_id.as_deref().unwrap_or("legacy"),
        "frame-extraction",
        &key,
        cancel.clone(),
    )
    .await?;
    ensure_chunk_active(&storage, chunk)?;
    check_processing_cancel(&cancel)?;
    if let Some(frames) = cache()
        .lock()
        .map_err(|_| {
            ai_error(
                "ai_processing_worker_state_failed",
                "Frame cache unavailable.",
            )
        })?
        .iter()
        .find(|entry| entry.0 == key)
        .map(|entry| entry.2.clone())
    {
        return Ok(frames);
    }
    let worker_app = app.clone();
    let mut worker_chunk = chunk.clone();
    // The cache owns a unique directory, never a sibling worker's cleanup path.
    worker_chunk.id = format!("shared-{key}-{}", Uuid::now_v7());
    let frames = tauri::async_runtime::spawn_blocking(move || {
        let _lease = lease;
        extract_full_frames(&worker_app, &worker_chunk, &segment, offset, fps, &cancel)
    })
    .await
    .map_err(|_| {
        ai_error(
            "ai_ocr_extraction_worker_failed",
            "Frame extraction stopped unexpectedly.",
        )
    })??;
    ensure_chunk_active(&storage, chunk)?;
    let frames = Arc::new(frames);
    let bytes = frames.file_bytes();
    if bytes <= MAX_BYTES {
        let mut entries = cache().lock().map_err(|_| {
            ai_error(
                "ai_processing_worker_state_failed",
                "Frame cache unavailable.",
            )
        })?;
        while entries.len() >= MAX_ENTRIES
            || entries.iter().map(|entry| entry.3).sum::<u64>() + bytes > MAX_BYTES
        {
            entries.pop_front();
        }
        entries.push_back((key, chunk.job_id.clone(), frames.clone(), bytes));
    }
    Ok(frames)
}
