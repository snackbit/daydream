use super::*;

pub(super) async fn run_frame(
    app: &AppHandle,
    chunk: &ProcessingChunkRow,
    frame: &FullFrame,
    runtime: Option<&ResolvedTesseractRuntime>,
    vision: Option<&VisionRequestConfig>,
    cancel: Arc<AtomicBool>,
    lease: crate::recording_privacy::ProcessingLease,
) -> Result<OcrContentInput, AiError> {
    ensure_chunk_active(&app.state::<AppStorage>(), chunk)?;
    check_processing_cancel(&cancel)?;
    if let Some(runtime) = runtime {
        let settings =
            settings::load_app_settings(&app.state::<AppStorage>()).map_err(settings_error)?;
        app.state::<AppStorage>()
            .with_connection(|connection| {
                storage::set_processing_chunk_phase(connection, &chunk.id, "waiting-for-worker")
            })
            .map_err(storage_error)?;
        let _permit = super::super::visual::acquire_ocr_permit_for_documents(
            super::super::visual::local_ocr_limit(&settings.ai.processing),
            cancel.clone(),
        )
        .await?;
        ensure_chunk_active(&app.state::<AppStorage>(), chunk)?;
        check_processing_cancel(&cancel)?;
        app.state::<AppStorage>()
            .with_connection(|connection| {
                storage::set_processing_chunk_phase(connection, &chunk.id, "recognizing-text")
            })
            .map_err(storage_error)?;
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
            let _lease = lease;
            tesseract::run_tesseract_document(&runtime, &owned, &cancel)
        })
        .await
        .map_err(|_| ai_error("ai_ocr_tesseract_worker_failed", "Tesseract worker failed."))?
    } else if let Some(vision) = vision {
        let settings =
            settings::load_app_settings(&app.state::<AppStorage>()).map_err(settings_error)?;
        let _permit = super::super::visual::acquire_vlm_permit_for_documents(
            if settings.ai.processing.resource_mode == "background" {
                1
            } else {
                settings.ai.processing.max_parallel_vlm_jobs as usize
            },
            cancel.clone(),
        )
        .await?;
        ensure_chunk_active(&app.state::<AppStorage>(), chunk)?;
        check_processing_cancel(&cancel)?;
        super::super::visual::verify_current_vision_configuration(
            &app.state::<AppStorage>(),
            vision,
            true,
        )?;
        if super::super::visual::shared_inference::matching_observation(
            &app.state::<AppStorage>(),
            chunk,
            vision,
            frame.timestamp_ms,
        )? {
            super::super::visual::shared_inference::infer(
                app, chunk, frame, vision, true, true, cancel,
            )
            .await?
            .ocr
            .ok_or_else(|| {
                ai_error(
                    "ai_ocr_vision_response_invalid",
                    "Vision OCR returned an invalid literal document.",
                )
            })
        } else {
            vision::run_vision_document(vision, frame, cancel).await
        }
    } else {
        Err(ai_error(
            "ai_ocr_config_invalid",
            "OCR engine is unconfigured.",
        ))
    }
}

pub(super) fn raw_word_inputs(
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
