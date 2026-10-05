use super::super::*;
use super::visual::VisionRequestConfig;
use crate::storage::{OcrContentInput, OcrFrameInput};

mod config;
mod extraction;
mod reprocess;
mod tesseract;
mod vision;
pub(crate) use config::capture_ocr_job_config;
use config::{load_ocr_job_config, OcrEngine};
use extraction::{extract_full_frames, FullFrame};
pub(crate) use reprocess::reprocess_recording_ocr;

pub(in crate::ai) async fn process_ocr_document_chunk(
    app: &AppHandle,
    chunk: &ProcessingChunkRow,
    cancel: Arc<AtomicBool>,
) -> Result<(), AiError> {
    let storage = app.state::<AppStorage>();
    ensure_chunk_active(&storage, chunk)?;
    let config = load_ocr_job_config(&storage, &chunk.job_id)?;
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
            Ok((segment, offset, recording.default_fps))
        })
        .map_err(storage_error)?;
    let source_track_id = crate::recording_privacy::source_track_id(&segment, "video")
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
    let frames = extract_full_frames(
        app,
        chunk,
        &segment,
        offset,
        config.fps(recording_fps),
        &cancel,
    )?;
    for index in 0..frames.len() {
        check_processing_cancel(&cancel)?;
        ensure_chunk_active(&storage, chunk)?;
        let frame = frames.frame(index)?;
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
        let result = infer_or_reuse_frame(&storage, chunk, &mut input, || {
            run_frame(
                app,
                chunk,
                &frame,
                runtime.as_ref(),
                vision_config.as_ref(),
                cancel.clone(),
            )
        })
        .await;
        match result {
            Ok(mut content) => {
                check_processing_cancel(&cancel)?;
                ensure_chunk_active(&storage, chunk)?;
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
                            0.0,
                            Some(&error.code),
                            0,
                            now_ms(),
                        )
                    })
                    .map_err(storage_error)?;
                emit_processing_state(app, &chunk.recording_id);
                return Err(error);
            }
        }
        storage
            .with_connection(|connection| {
                storage::update_processing_chunk_progress(
                    connection,
                    &chunk.id,
                    ((index + 1) as f64 / frames.len() as f64).min(0.99),
                    now_ms(),
                )
            })
            .map_err(storage_error)?;
        emit_processing_state(app, &chunk.recording_id);
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

async fn run_frame(
    app: &AppHandle,
    chunk: &ProcessingChunkRow,
    frame: &FullFrame,
    runtime: Option<&ResolvedTesseractRuntime>,
    vision: Option<&VisionRequestConfig>,
    cancel: Arc<AtomicBool>,
) -> Result<OcrContentInput, AiError> {
    ensure_chunk_active(&app.state::<AppStorage>(), chunk)?;
    check_processing_cancel(&cancel)?;
    if let Some(runtime) = runtime {
        let settings =
            settings::load_app_settings(&app.state::<AppStorage>()).map_err(settings_error)?;
        let _permit = super::visual::acquire_ocr_permit_for_documents(
            settings.ai.processing.max_parallel_ocr_jobs as usize,
        )
        .await;
        ensure_chunk_active(&app.state::<AppStorage>(), chunk)?;
        check_processing_cancel(&cancel)?;
        let runtime = runtime.clone();
        let owned = FullFrame {
            path: frame.path.clone(),
            timestamp_ms: frame.timestamp_ms,
            end_ms: frame.end_ms,
            width: frame.width,
            height: frame.height,
            pixel_hash: frame.pixel_hash.clone(),
            source_ref: frame.source_ref.clone(),
        };
        tauri::async_runtime::spawn_blocking(move || {
            tesseract::run_tesseract_document(&runtime, &owned)
        })
        .await
        .map_err(|_| ai_error("ai_ocr_tesseract_worker_failed", "Tesseract worker failed."))?
    } else if let Some(vision) = vision {
        let settings =
            settings::load_app_settings(&app.state::<AppStorage>()).map_err(settings_error)?;
        let _permit = super::visual::acquire_vlm_permit_for_documents(
            settings.ai.processing.max_parallel_vlm_jobs as usize,
        )
        .await;
        ensure_chunk_active(&app.state::<AppStorage>(), chunk)?;
        check_processing_cancel(&cancel)?;
        vision::run_vision_document(vision, frame, cancel).await
    } else {
        Err(ai_error(
            "ai_ocr_config_invalid",
            "OCR engine is unconfigured.",
        ))
    }
}

fn raw_word_inputs(
    chunk: &ProcessingChunkRow,
    frame: &FullFrame,
    content: &mut OcrContentInput,
) -> Vec<CreateVisualRegionInput> {
    let mut rows = Vec::new();
    for block in &mut content.blocks {
        block.raw_region_ids.clear();
        for word in &mut block.words {
            let Some(region) = &word.region else {
                continue;
            };
            let id = Uuid::now_v7().to_string();
            word.raw_region_id = Some(id.clone());
            block.raw_region_ids.push(id.clone());
            rows.push(CreateVisualRegionInput {
                chunk_id: chunk.id.clone(),
                confidence: word.confidence,
                frame_timestamp_ms: frame.timestamp_ms,
                id,
                job_id: chunk.job_id.clone(),
                model: Some("tesseract".into()),
                observation_kind: None,
                prompt_version: None,
                provider: "tesseract".into(),
                recording_id: chunk.recording_id.clone(),
                region_height: region.height,
                region_width: region.width,
                region_x: region.x,
                region_y: region.y,
                source_frame_ref: frame.source_ref.clone(),
                text: word.text.clone(),
            });
        }
    }
    rows
}
