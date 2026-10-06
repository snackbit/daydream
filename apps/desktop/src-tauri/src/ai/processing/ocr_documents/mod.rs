use super::super::*;
use super::visual::VisionRequestConfig;
use crate::storage::{OcrContentInput, OcrFrameInput};
use std::collections::BTreeSet;

mod config;
mod extraction;
mod recognition;
mod recognition_lease;
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

pub(in crate::ai) async fn process_ocr_document_chunk(
    app: &AppHandle,
    chunk: &ProcessingChunkRow,
    cancel: Arc<AtomicBool>,
    lease: crate::recording_privacy::ProcessingLease,
) -> Result<(), AiError> {
    let storage = app.state::<AppStorage>();
    ensure_chunk_active(&storage, chunk)?;
    let config = load_chunk_ocr_config(&storage, chunk)?;
    let (segment, offset, recording_fps) = storage
        .with_connection(|connection| {
            let recording = storage::get_recording(connection, &chunk.recording_id)?;
            let segments = storage::list_recording_segments(connection, &chunk.recording_id)?;
            let id = chunk
                .segment_id
                .as_deref()
                .ok_or(rusqlite::Error::InvalidQuery)?;
            let offset = playback_offset_for_segment(&segments, id)
                .ok_or(rusqlite::Error::QueryReturnedNoRows)?;
            let segment = segments
                .into_iter()
                .find(|segment| segment.id == id)
                .ok_or(rusqlite::Error::QueryReturnedNoRows)?;
            Ok((
                source_segment(connection, chunk, segment)?,
                offset,
                recording.default_fps,
            ))
        })
        .map_err(storage_error)?;
    let source_track_id = chunk
        .track_id
        .clone()
        .or_else(|| crate::recording_privacy::source_track_id(&segment, "video"))
        .ok_or_else(|| ai_error("ai_ocr_source_invalid", "OCR source track is missing."))?;
    let runtime = if config.engine == OcrEngine::Tesseract {
        Some(resolve_tesseract_runtime(app)?)
    } else {
        None
    };
    let vision_config = if config.engine == OcrEngine::Vision {
        let mut settings = settings::default_settings();
        settings.ai.vision = config.vision.clone().ok_or_else(|| {
            ai_error(
                "ai_ocr_config_invalid",
                "Vision OCR configuration is missing.",
            )
        })?;
        Some(super::visual::vision_request_config_for_ocr(
            &storage, &settings,
        )?)
    } else {
        None
    };
    let runtime_identity = runtime
        .as_ref()
        .map(tesseract::runtime_identity)
        .transpose()?
        .unwrap_or_else(|| "vision".into());
    let fingerprint = config.fingerprint(&runtime_identity)?;
    let extraction_app = app.clone();
    let extraction_chunk = chunk.clone();
    let extraction_cancel = cancel.clone();
    let fps = config.fps(recording_fps);
    let extraction_lease = lease.clone();
    let frames = tauri::async_runtime::spawn_blocking(move || {
        let _lease = extraction_lease;
        extract_full_frames(
            &extraction_app,
            &extraction_chunk,
            &segment,
            offset,
            fps,
            &extraction_cancel,
        )
    })
    .await
    .map_err(|_| {
        ai_error(
            "ai_ocr_extraction_worker_failed",
            "OCR frame extraction stopped unexpectedly.",
        )
    })??;
    let frames = Arc::new(frames);
    let selection = super::worker::load_unit_selection(&storage, &chunk.id)?;
    let successful=storage.with_connection(|connection| {
        let mut statement=connection.prepare("SELECT frame_timestamp_ms FROM ocr_frame_occurrences WHERE chunk_id=?1 AND status='succeeded' AND attempt_count=?2")?;
        let rows=statement.query_map(rusqlite::params![chunk.id,chunk.attempt_count],|row|row.get::<_,i64>(0))?;
        rows.collect::<rusqlite::Result<BTreeSet<_>>>()
    }).map_err(storage_error)?;
    storage
        .with_connection(|connection| {
            storage::set_processing_chunk_total(connection, &chunk.id, frames.len() as i64)
        })
        .map_err(storage_error)?;
    storage
        .with_connection(|connection| {
            storage::set_processing_chunk_phase(connection, &chunk.id, "recognizing-text")
        })
        .map_err(storage_error)?;
    let settings = settings::load_app_settings(&storage).map_err(settings_error)?;
    let parallel = if settings.ai.processing.resource_mode == "background" {
        1
    } else if config.engine == OcrEngine::Vision {
        settings.ai.processing.max_parallel_vlm_jobs as usize
    } else {
        settings.ai.processing.max_parallel_ocr_jobs as usize
    }
    .clamp(1, 8);
    let completed = AtomicUsize::new(0);
    let source_failure = Mutex::<Option<String>>::new(None);
    let results = futures_util::stream::iter(0..frames.len())
        .map(|index| {
            let frames = frames.clone();
            let cancel = cancel.clone();
            let storage = &storage;
            let config = &config;
            let successful = &successful;
            let selection = &selection;
            let source_track_id = &source_track_id;
            let fingerprint = &fingerprint;
            let runtime = &runtime;
            let vision_config = &vision_config;
            let completed = &completed;
            let source_failure = &source_failure;
            let lease = lease.clone();
            async move {
                check_processing_cancel(&cancel)?;
                ensure_chunk_active(storage, chunk)?;
                let timestamp = frames.timestamp(index);
                if successful.contains(&timestamp)
                    || selection
                        .frame_timestamps
                        .as_ref()
                        .is_some_and(|timestamps| !timestamps.contains(&timestamp))
                {
                    completed.fetch_add(1, Ordering::SeqCst);
                    return Ok(());
                }
                let frame_set = frames.clone();
                let decode_lease = lease.clone();
                let frame = tauri::async_runtime::spawn_blocking(move || {
                    let _lease = decode_lease;
                    frame_set.frame(index)
                })
                .await
                .map_err(|_| {
                    ai_error(
                        "ai_ocr_decode_worker_failed",
                        "The selected frame could not be decoded.",
                    )
                })??;
                let mut input = OcrFrameInput {
                    id: Uuid::now_v7().to_string(),
                    frame_timestamp_ms: frame.timestamp_ms,
                    sample_end_ms: frame.end_ms,
                    source_frame_ref: frame.source_ref.clone(),
                    source_track_id: source_track_id.clone(),
                    width: frame.width,
                    height: frame.height,
                    config_fingerprint: fingerprint.clone(),
                    engine: if config.engine == OcrEngine::Tesseract {
                        "tesseract"
                    } else {
                        "vision"
                    }
                    .into(),
                    provider: if config.engine == OcrEngine::Tesseract {
                        "tesseract"
                    } else {
                        "openai-compatible"
                    }
                    .into(),
                    model: vision_config
                        .as_ref()
                        .map(|config| config.model.clone())
                        .or_else(|| Some("tesseract".into())),
                    pixel_hash: frame.pixel_hash.clone(),
                    status: "succeeded".into(),
                    error_code: None,
                    content: None,
                    cache_content_id: None,
                    raw_regions: Vec::new(),
                };
                let frame_state = storage
                    .with_connection(|connection| {
                        storage::upsert_visual_frame_processing_state(
                            connection,
                            &CreateVisualFrameProcessingStateInput {
                                id: Uuid::now_v7().to_string(),
                                chunk_id: chunk.id.clone(),
                                created_at_ms: now_ms(),
                                error_code: None,
                                frame_timestamp_ms: frame.timestamp_ms,
                                job_id: chunk.job_id.clone(),
                                output_count: 0,
                                progress: 0.5,
                                recording_id: chunk.recording_id.clone(),
                                region_height: frame.height,
                                region_width: frame.width,
                                region_x: 0,
                                region_y: 0,
                                segment_id: chunk.segment_id.clone(),
                                source_frame_ref: frame.source_ref.clone(),
                                source_kind: "ocr".into(),
                                status: "running".into(),
                                updated_at_ms: now_ms(),
                            },
                        )
                    })
                    .map_err(storage_error)?;
                let _recognition = recognition_lease::acquire(
                    &chunk.recording_id,
                    source_track_id,
                    &frame.pixel_hash,
                    fingerprint,
                    cancel.clone(),
                )
                .await?;
                let blocked = source_failure
                    .lock()
                    .map_err(|_| {
                        ai_error(
                            "ai_processing_worker_state_failed",
                            "Processing state is unavailable.",
                        )
                    })?
                    .clone();
                let result = if let Some(code) = blocked {
                    Err(ai_error(
                        &code,
                        "This source needs its provider configuration repaired.",
                    ))
                } else {
                    infer_or_reuse_frame(storage, chunk, &mut input, || {
                        run_frame(
                            app,
                            chunk,
                            &frame,
                            runtime.as_ref(),
                            vision_config.as_ref(),
                            cancel.clone(),
                            lease.clone(),
                        )
                    })
                    .await
                };
                match result {
                    Ok(mut content) => {
                        check_processing_cancel(&cancel)?;
                        ensure_chunk_active(storage, chunk)?;
                        input.raw_regions = raw_word_inputs(chunk, &frame, &mut content);
                        input.content = Some(content);
                        let saved = storage
                            .with_connection_mut(|connection| {
                                storage::save_ocr_frame_result(connection, chunk, &input)
                            })
                            .map_err(storage_error)?;
                        if !saved {
                            return Err(ai_error(
                                "ai_processing_cancelled",
                                "OCR source is no longer eligible.",
                            ));
                        }
                        let output_count = input
                            .content
                            .as_ref()
                            .map(|content| {
                                if input.raw_regions.is_empty() {
                                    content.blocks.len()
                                } else {
                                    input.raw_regions.len()
                                }
                            })
                            .unwrap_or(0);
                        storage
                            .with_connection(|connection| {
                                storage::update_visual_frame_processing_state_status(
                                    connection,
                                    &frame_state.id,
                                    "succeeded",
                                    1.0,
                                    None,
                                    output_count as i64,
                                    now_ms(),
                                )
                            })
                            .map_err(storage_error)?;
                    }
                    Err(error) => {
                        if error.code == "ai_processing_cancelled" {
                            return Err(error);
                        }
                        if error.code.ends_with("auth_failed")
                            || error.code.ends_with("model_not_found")
                            || error.code.ends_with("request_invalid")
                            || error.code.ends_with("configuration_changed")
                            || error.code.ends_with("unconfigured")
                        {
                            *source_failure.lock().map_err(|_| {
                                ai_error(
                                    "ai_processing_worker_state_failed",
                                    "Processing state is unavailable.",
                                )
                            })? = Some(error.code.clone());
                        }
                        input.status = "failed".into();
                        input.error_code = Some(error.code.clone());
                        storage
                            .with_connection_mut(|connection| {
                                storage::save_ocr_frame_result(connection, chunk, &input)
                            })
                            .map_err(storage_error)?;
                        storage
                            .with_connection(|connection| {
                                storage::update_visual_frame_processing_state_status(
                                    connection,
                                    &frame_state.id,
                                    "failed",
                                    1.0,
                                    Some(&error.code),
                                    0,
                                    now_ms(),
                                )
                            })
                            .map_err(storage_error)?;
                        emit_processing_state(app, &chunk.recording_id);
                        // Persist the failure, then continue to later samples.
                    }
                }
                storage
                    .with_connection(|connection| {
                        storage::update_processing_chunk_progress(
                            connection,
                            &chunk.id,
                            ((completed.fetch_add(1, Ordering::SeqCst) + 1) as f64
                                / frames.len() as f64)
                                .min(0.99),
                            now_ms(),
                        )
                    })
                    .map_err(storage_error)?;
                emit_processing_state(app, &chunk.recording_id);
                Ok::<(), AiError>(())
            }
        })
        .buffer_unordered(parallel)
        .collect::<Vec<_>>()
        .await;
    for result in results {
        result?;
    }
    if let Some(code) = storage
        .with_connection(|connection| {
            storage::first_processing_chunk_failure(connection, &chunk.id)
        })
        .map_err(storage_error)?
    {
        return Err(ai_error(
            &code,
            "Some OCR frames failed. Successful frames were saved.",
        ));
    }
    Ok(())
}

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
