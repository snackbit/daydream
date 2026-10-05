use super::*;

fn log_visual_frame_started(
    chunk: &ProcessingChunkRow,
    frame_index: usize,
    frame: &ExtractedVisualFrame,
) {
    tracing::debug!(
        recording_id = %chunk.recording_id,
        job_id = %chunk.job_id,
        chunk_id = %chunk.id,
        source_kind = %chunk.source_kind,
        frame_index,
        frame_timestamp_ms = frame.frame_timestamp_ms,
        source_frame_ref = %frame.source_frame_ref,
        region_x = frame.region.x,
        region_y = frame.region.y,
        region_width = frame.region.width,
        region_height = frame.region.height,
        full_checkpoint = frame.full_checkpoint,
        abrupt_transition = frame.abrupt_transition,
        "ai_processing_visual_frame_started"
    );
}

pub(super) struct ProcessedOcrFrame {
    pub(super) frame: ExtractedVisualFrame,
    pub(super) frame_index: usize,
    pub(super) ocr_elapsed_ms: u64,
    pub(super) regions: Vec<OcrCandidate>,
}

pub(super) struct ProcessedVlmFrame {
    pub(super) frame_index: usize,
    pub(super) observations: Vec<PendingVlmObservation>,
    pub(super) ocr_candidate_count: usize,
    pub(super) ocr_elapsed_ms: u64,
    pub(super) vlm_elapsed_ms: u64,
}

pub(super) async fn process_ocr_frames(
    app: &AppHandle,
    storage: &AppStorage,
    chunk: &ProcessingChunkRow,
    frames: Vec<ExtractedVisualFrame>,
    max_parallel_ocr_jobs: usize,
    cancel: Arc<AtomicBool>,
) -> Result<Vec<ProcessedOcrFrame>, AiError> {
    let total_frame_count = frames.len();
    let frame_states = queue_visual_frame_states(app, storage, chunk, &frames)?;
    let completed_count = Arc::new(AtomicUsize::new(0));
    let local_limit = max_parallel_ocr_jobs.max(1);
    let results = futures_util::stream::iter(frames.into_iter().enumerate().zip(frame_states))
        .map(|((frame_index, frame), frame_state)| {
            let app = app.clone();
            let chunk = chunk.clone();
            let cancel = cancel.clone();
            let completed_count = completed_count.clone();
            async move {
                check_processing_cancel(&cancel)?;
                let _permit = acquire_ocr_permit(local_limit).await;
                check_processing_cancel(&cancel)?;
                let storage = app.state::<AppStorage>();
                ensure_chunk_active(&storage, &chunk)?;
                mark_visual_frame_state(
                    &app,
                    &storage,
                    &chunk,
                    &frame_state.id,
                    VisualFrameUpdate::Running,
                )?;
                log_visual_frame_started(&chunk, frame_index, &frame);
                let ocr_started = Instant::now();
                let app_for_ocr = app.clone();
                let frame_for_ocr = frame.clone();
                let regions_result = tauri::async_runtime::spawn_blocking(move || {
                    run_tesseract_ocr(&app_for_ocr, frame_for_ocr.path(), &frame_for_ocr)
                })
                .await
                .map_err(|error| {
                    ai_error(
                        "ai_ocr_tesseract_worker_failed",
                        &format!("Tesseract OCR worker failed: {error}"),
                    )
                })?;
                let frame_ocr_elapsed_ms = elapsed_ms(ocr_started);
                match regions_result {
                    Ok(regions) => {
                        mark_visual_frame_state(
                            &app,
                            &storage,
                            &chunk,
                            &frame_state.id,
                            VisualFrameUpdate::Succeeded(regions.len()),
                        )?;
                        let completed = completed_count.fetch_add(1, Ordering::SeqCst) + 1;
                        update_visual_chunk_progress(
                            &app,
                            &storage,
                            &chunk,
                            completed,
                            total_frame_count,
                        )?;
                        tracing::info!(
                            recording_id = %chunk.recording_id,
                            job_id = %chunk.job_id,
                            chunk_id = %chunk.id,
                            source_kind = %chunk.source_kind,
                            frame_index,
                            frame_timestamp_ms = frame.frame_timestamp_ms,
                            ocr_candidate_count = regions.len(),
                            ocr_elapsed_ms = frame_ocr_elapsed_ms,
                            "ai_processing_visual_frame_completed"
                        );
                        Ok(ProcessedOcrFrame {
                            frame,
                            frame_index,
                            ocr_elapsed_ms: frame_ocr_elapsed_ms,
                            regions,
                        })
                    }
                    Err(error) => {
                        let _ = mark_visual_frame_state(
                            &app,
                            &storage,
                            &chunk,
                            &frame_state.id,
                            VisualFrameUpdate::Failed(&error.code),
                        );
                        Err(error)
                    }
                }
            }
        })
        .buffer_unordered(local_limit)
        .collect::<Vec<Result<ProcessedOcrFrame, AiError>>>()
        .await;
    let mut output = Vec::with_capacity(results.len());
    let mut first_error = None;
    for result in results {
        match result {
            Ok(frame) => output.push(frame),
            Err(error) if first_error.is_none() => first_error = Some(error),
            Err(_) => {}
        }
    }
    if let Some(error) = first_error {
        return Err(error);
    }
    output.sort_by_key(|frame| frame.frame_index);
    Ok(output)
}

pub(super) struct VlmFrameBatch<'a> {
    pub(super) config: &'a VisionRequestConfig,
    pub(super) frames: Vec<ExtractedVisualFrame>,
    pub(super) persisted_ocr_candidates: BTreeMap<i64, Vec<OcrCandidate>>,
    pub(super) previous_observations: Vec<PreviousVisualObservation>,
    pub(super) max_parallel_ocr_jobs: usize,
    pub(super) max_parallel_vlm_jobs: usize,
    pub(super) cancel: Arc<AtomicBool>,
}

pub(super) async fn process_vlm_frames(
    app: &AppHandle,
    storage: &AppStorage,
    chunk: &ProcessingChunkRow,
    batch: VlmFrameBatch<'_>,
) -> Result<Vec<ProcessedVlmFrame>, AiError> {
    let VlmFrameBatch {
        config,
        frames,
        persisted_ocr_candidates,
        previous_observations,
        max_parallel_ocr_jobs,
        max_parallel_vlm_jobs,
        cancel,
    } = batch;
    let total_frame_count = frames.len();
    let frame_states = queue_visual_frame_states(app, storage, chunk, &frames)?;
    let completed_count = Arc::new(AtomicUsize::new(0));
    let local_limit = max_parallel_vlm_jobs.max(1);
    let persisted_ocr_candidates = Arc::new(persisted_ocr_candidates);
    let previous_observations = Arc::new(previous_observations);
    let config = Arc::new(config.clone());
    let results = futures_util::stream::iter(frames.into_iter().enumerate().zip(frame_states))
        .map(|((frame_index, frame), frame_state)| {
            let app = app.clone();
            let chunk = chunk.clone();
            let cancel = cancel.clone();
            let completed_count = completed_count.clone();
            let config = config.clone();
            let persisted_ocr_candidates = persisted_ocr_candidates.clone();
            let previous_observations = previous_observations.clone();
            async move {
                check_processing_cancel(&cancel)?;
                let storage = app.state::<AppStorage>();
                let _vlm_permit = acquire_vlm_permit(max_parallel_vlm_jobs.max(1)).await;
                check_processing_cancel(&cancel)?;
                mark_visual_frame_state(
                    &app,
                    &storage,
                    &chunk,
                    &frame_state.id,
                    VisualFrameUpdate::Running,
                )?;
                log_visual_frame_started(&chunk, frame_index, &frame);
                let ocr_started = Instant::now();
                let (ocr_regions, ocr_source) = vlm_ocr_context_for_frame(
                    &app,
                    &frame,
                    &persisted_ocr_candidates,
                    max_parallel_ocr_jobs,
                    cancel.clone(),
                )
                .await;
                let frame_ocr_elapsed_ms = elapsed_ms(ocr_started);
                let recent_observations =
                    recent_visual_observations(&previous_observations, frame.frame_timestamp_ms);
                check_processing_cancel(&cancel)?;
                ensure_chunk_active(&storage, &chunk)?;
                let vlm_started = Instant::now();
                let observations =
                    request_vlm_observations(&config, &frame, &ocr_regions, &recent_observations)
                        .await;
                let frame_vlm_elapsed_ms = elapsed_ms(vlm_started);
                match observations {
                    Ok(observations) => {
                        mark_visual_frame_state(
                            &app,
                            &storage,
                            &chunk,
                            &frame_state.id,
                            VisualFrameUpdate::Succeeded(observations.len()),
                        )?;
                        let completed = completed_count.fetch_add(1, Ordering::SeqCst) + 1;
                        update_visual_chunk_progress(
                            &app,
                            &storage,
                            &chunk,
                            completed,
                            total_frame_count,
                        )?;
                        tracing::info!(
                            recording_id = %chunk.recording_id,
                            job_id = %chunk.job_id,
                            chunk_id = %chunk.id,
                            source_kind = %chunk.source_kind,
                            frame_index,
                            frame_timestamp_ms = frame.frame_timestamp_ms,
                            source_frame_ref = %frame.source_frame_ref,
                            ocr_source,
                            ocr_candidate_count = ocr_regions.len(),
                            recent_observation_count = recent_observations.len(),
                            vlm_observation_count = observations.len(),
                            ocr_elapsed_ms = frame_ocr_elapsed_ms,
                            vlm_elapsed_ms = frame_vlm_elapsed_ms,
                            "ai_processing_visual_frame_completed"
                        );
                        Ok(ProcessedVlmFrame {
                            frame_index,
                            observations,
                            ocr_candidate_count: ocr_regions.len(),
                            ocr_elapsed_ms: frame_ocr_elapsed_ms,
                            vlm_elapsed_ms: frame_vlm_elapsed_ms,
                        })
                    }
                    Err(error) => {
                        let _ = mark_visual_frame_state(
                            &app,
                            &storage,
                            &chunk,
                            &frame_state.id,
                            VisualFrameUpdate::Failed(&error.code),
                        );
                        Err(error)
                    }
                }
            }
        })
        .buffer_unordered(local_limit)
        .collect::<Vec<Result<ProcessedVlmFrame, AiError>>>()
        .await;
    let mut output = Vec::with_capacity(results.len());
    let mut first_error = None;
    for result in results {
        match result {
            Ok(frame) => output.push(frame),
            Err(error) if first_error.is_none() => first_error = Some(error),
            Err(_) => {}
        }
    }
    if let Some(error) = first_error {
        return Err(error);
    }
    output.sort_by_key(|frame| frame.frame_index);
    Ok(output)
}

async fn vlm_ocr_context_for_frame(
    app: &AppHandle,
    frame: &ExtractedVisualFrame,
    persisted_ocr_candidates: &BTreeMap<i64, Vec<OcrCandidate>>,
    max_parallel_ocr_jobs: usize,
    cancel: Arc<AtomicBool>,
) -> (Vec<OcrCandidate>, &'static str) {
    if let Some(regions) = persisted_ocr_candidates
        .get(&frame.frame_timestamp_ms)
        .filter(|regions| !regions.is_empty())
    {
        return (regions.clone(), "persisted");
    }

    let _permit = acquire_ocr_permit(max_parallel_ocr_jobs.max(1)).await;
    if check_processing_cancel(&cancel).is_err() {
        return (Vec::new(), "missing");
    }
    let app_for_ocr = app.clone();
    let frame_for_ocr = frame.clone();
    let regions = tauri::async_runtime::spawn_blocking(move || {
        run_tesseract_ocr(&app_for_ocr, frame_for_ocr.path(), &frame_for_ocr)
    })
    .await
    .ok()
    .and_then(Result::ok);

    match regions {
        Some(regions) => (regions, "fallback-tesseract"),
        None => (Vec::new(), "missing"),
    }
}

fn update_visual_chunk_progress(
    app: &AppHandle,
    storage: &AppStorage,
    chunk: &ProcessingChunkRow,
    completed_frame_count: usize,
    total_frame_count: usize,
) -> Result<(), AiError> {
    let progress = if total_frame_count == 0 {
        1.0
    } else {
        (completed_frame_count as f64 / total_frame_count as f64).clamp(0.05, 0.99)
    };
    storage
        .with_connection(|connection| {
            storage::update_processing_chunk_progress(connection, &chunk.id, progress, now_ms())
                .map(|_row| ())
        })
        .map_err(storage_error)?;
    emit_processing_state(app, &chunk.recording_id);
    Ok(())
}
