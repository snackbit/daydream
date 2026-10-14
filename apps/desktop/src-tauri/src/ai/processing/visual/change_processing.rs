use super::super::ocr_documents::{
    extraction::{FullFrame, FullFrameSet},
    frame_cache, sampling,
};
use super::change_protocol::{ChangeStatus, VERSION};
use super::*;

#[cfg(test)]
#[path = "change_processing_tests.rs"]
mod tests;

pub(super) async fn process(
    app: &AppHandle,
    chunk: &ProcessingChunkRow,
    cancel: Arc<AtomicBool>,
    lease: crate::recording_privacy::ProcessingLease,
) -> Result<(), AiError> {
    let storage = app.state::<AppStorage>();
    let settings = configuration::load_visual_settings(&storage, chunk)?;
    let config = vision_request_config(&storage, &settings)?;
    let (segment, base, fps) = storage
        .with_connection(|connection| {
            let recording = storage::get_recording(connection, &chunk.recording_id)?;
            let rows = storage::list_recording_segments(connection, &chunk.recording_id)?;
            let id = chunk
                .segment_id
                .as_deref()
                .ok_or(rusqlite::Error::InvalidQuery)?;
            let base =
                playback_offset_for_segment(&rows, id).ok_or(rusqlite::Error::InvalidQuery)?;
            let segment = rows
                .into_iter()
                .find(|row| row.id == id)
                .ok_or(rusqlite::Error::InvalidQuery)?;
            let segment = source_segment(connection, chunk, segment)?;
            let fps = if settings.ai.processing.frame_recognition_sync_with_recording {
                recording.default_fps
            } else {
                settings.ai.processing.frame_recognition_fps
            };
            Ok((segment, base, fps))
        })
        .map_err(storage_error)?;
    let adaptive = settings.ai.processing.visual_sampling_mode == "adaptive-v1";
    let fps = if adaptive { fps.min(0.2) } else { fps };
    let frames = if let Some(frames) = cached_frames(&storage, chunk, &config, fps, adaptive)? {
        ObservationFrames::Cached(frames)
    } else {
        ObservationFrames::Extracted(
            frame_cache::extract(
                app,
                chunk,
                segment,
                base,
                fps,
                cancel.clone(),
                lease.clone(),
            )
            .await?,
        )
    };
    let selected = if adaptive {
        if let ObservationFrames::Extracted(extracted) = &frames {
            let worker_app = app.clone();
            let worker_chunk = chunk.clone();
            let worker_frames = extracted.clone();
            let worker_cancel = cancel.clone();
            let worker_lease = lease.clone();
            let fingerprint = shared_inference::fingerprint(&config);
            Some(
                tauri::async_runtime::spawn_blocking(move || {
                    let _lease = worker_lease;
                    sampling::select(
                        &worker_app,
                        &worker_chunk,
                        &worker_frames,
                        &fingerprint,
                        true,
                        &worker_cancel,
                    )
                })
                .await
                .map_err(|_| {
                    ai_error("ai_ocr_decode_worker_failed", "Sampling worker stopped.")
                })??,
            )
        } else {
            None
        }
    } else {
        None
    };
    let selection = super::super::worker::load_unit_selection(&storage, &chunk.id)?;
    let succeeded = storage.with_connection(|connection| {
        let mut query = connection.prepare("SELECT frame_timestamp_ms FROM visual_frame_processing_states WHERE chunk_id=?1 AND status='succeeded'")?;
        let rows = query.query_map([&chunk.id], |row| row.get::<_,i64>(0))?.collect::<rusqlite::Result<BTreeSet<_>>>()?;
        Ok(rows)
    }).map_err(storage_error)?;
    let pending = prepare_observation_frames(
        app,
        &storage,
        chunk,
        &frames,
        &selection.frame_timestamps,
        &succeeded,
        &cancel,
    )?;
    let mut failure = None;
    for (index, frame, state) in pending {
        check_processing_cancel(&cancel)?;
        ensure_chunk_active(&storage, chunk)?;
        let visual = visual_frame(&frame);
        if !should_observe(
            selected.as_deref(),
            index,
            selection.frame_timestamps.as_deref(),
        ) {
            mark_visual_frame_state(
                app,
                &storage,
                chunk,
                &state.id,
                VisualFrameUpdate::Succeeded(0),
            )?;
            super::frame_jobs::update_visual_chunk_progress(
                app,
                &storage,
                chunk,
                index + 1,
                frames.len(),
            )?;
            continue;
        }
        storage
            .with_connection(|connection| {
                storage::set_processing_chunk_phase(connection, &chunk.id, "waiting-for-model")
            })
            .map_err(storage_error)?;
        mark_visual_frame_state(app, &storage, chunk, &state.id, VisualFrameUpdate::Running)?;
        let _permit = acquire_vlm_cancellable(
            settings.ai.processing.max_parallel_vlm_jobs.max(1) as usize,
            cancel.clone(),
        )
        .await?;
        storage
            .with_connection(|connection| {
                storage::set_processing_chunk_phase(connection, &chunk.id, "observing-screen")
            })
            .map_err(storage_error)?;
        let result =
            shared_inference::infer(app, chunk, &frame, &config, false, true, cancel.clone()).await;
        match result {
            Ok(output) => {
                let Some(change) = output.change else {
                    let code = "ai_vision_remote_response_invalid";
                    mark_visual_frame_state(
                        app,
                        &storage,
                        chunk,
                        &state.id,
                        VisualFrameUpdate::Failed(code),
                    )?;
                    failure = Some(ai_error(
                        code,
                        "Screen comparison returned an invalid result.",
                    ));
                    continue;
                };
                let observations = if change.status == ChangeStatus::Unchanged {
                    Vec::new()
                } else {
                    vec![PendingVlmObservation {
                        confidence: None,
                        frame_timestamp_ms: frame.timestamp_ms,
                        observation_kind: "screen-observation".into(),
                        region: visual.region,
                        source_frame_ref: frame.source_ref.clone(),
                        text: change.text.unwrap_or_default(),
                    }]
                };
                let mut inputs = vlm_observation_inputs(chunk, &config, observations);
                for input in &mut inputs {
                    input.prompt_version = Some(VERSION.into());
                }
                let ids = inputs
                    .iter()
                    .map(|input| input.id.clone())
                    .collect::<Vec<_>>();
                persistence::persist_vlm_frame(&storage, chunk, &state.id, &visual, inputs)?;
                storage.with_connection(|connection| {
                    for id in &ids { connection.execute("UPDATE vlm_observations SET context_start_ms=?2,confirmed_end_ms=?3 WHERE id=?1", rusqlite::params![id,output.context_start,frame.timestamp_ms.saturating_add(1)])?; }
                    if change.status == ChangeStatus::Unchanged && !adaptive {
                        connection.execute("UPDATE vlm_observations SET confirmed_end_ms=?4 WHERE id=(SELECT o.id FROM vlm_observations o JOIN processing_chunks c ON c.id=o.chunk_id WHERE o.job_id=?1 AND c.track_id=?2 AND o.prompt_version=?5 AND o.confirmed_end_ms>=?7 AND o.frame_timestamp_ms=(SELECT frame_ms FROM studio_visual_samples WHERE job_id=?1 AND source_id=?2 AND fingerprint=?6 AND frame_ms<?3 AND json_extract(observation_json,'$.status') IN ('initial','changed') ORDER BY frame_ms DESC LIMIT 1))", rusqlite::params![chunk.job_id,chunk.track_id,frame.timestamp_ms,frame.timestamp_ms.saturating_add(1),VERSION,shared_inference::fingerprint(&config),output.context_start])?;
                    }
                    Ok(())
                }).map_err(storage_error)?;
            }
            Err(error) if error.code == "ai_processing_cancelled" => return Err(error),
            Err(error) => {
                mark_visual_frame_state(
                    app,
                    &storage,
                    chunk,
                    &state.id,
                    VisualFrameUpdate::Failed(&error.code),
                )?;
                failure = Some(error);
            }
        }
        super::frame_jobs::update_visual_chunk_progress(
            app,
            &storage,
            chunk,
            index + 1,
            frames.len(),
        )?;
        emit_processing_state(app, &chunk.recording_id);
    }
    if let Some(error) = failure {
        return Err(error);
    }
    Ok(())
}
fn should_observe(
    samples: Option<&[sampling::Sample]>,
    index: usize,
    retry_timestamps: Option<&[i64]>,
) -> bool {
    samples.is_none_or(|samples| samples[index].should_recognize(retry_timestamps))
}

fn prepare_observation_frames<R: tauri::Runtime>(
    app: &AppHandle<R>,
    storage: &AppStorage,
    chunk: &ProcessingChunkRow,
    frames: &ObservationFrames,
    retry_timestamps: &Option<Vec<i64>>,
    succeeded: &BTreeSet<i64>,
    cancel: &AtomicBool,
) -> Result<Vec<(usize, FullFrame, VisualFrameProcessingStateRow)>, AiError> {
    let mut pending = Vec::new();
    for index in 0..frames.len() {
        let timestamp = frames.timestamp(index);
        if succeeded.contains(&timestamp)
            || retry_timestamps
                .as_ref()
                .is_some_and(|timestamps| !timestamps.contains(&timestamp))
        {
            continue;
        }
        check_processing_cancel(cancel)?;
        ensure_chunk_active(storage, chunk)?;
        let frame = frames.frame(index)?;
        pending.push((index, frame));
    }
    let visuals = pending
        .iter()
        .map(|(_, frame)| visual_frame(frame))
        .collect::<Vec<_>>();
    let states = queue_visual_frame_states(app, storage, chunk, &visuals)?;
    Ok(pending
        .into_iter()
        .zip(states)
        .map(|((index, frame), state)| (index, frame, state))
        .collect())
}
fn visual_frame(frame: &FullFrame) -> ExtractedVisualFrame {
    ExtractedVisualFrame {
        abrupt_transition: false,
        before_context: None,
        after_context: None,
        frame_timestamp_ms: frame.timestamp_ms,
        full_checkpoint: true,
        height: frame.height,
        width: frame.width,
        path: frame.path.clone(),
        signature: frame.pixel_hash.clone(),
        source_frame_ref: frame.source_ref.clone(),
        region: VisualRegion {
            x: 0,
            y: 0,
            width: frame.width,
            height: frame.height,
        },
    }
}

enum ObservationFrames {
    Extracted(Arc<FullFrameSet>),
    Cached(Vec<FullFrame>),
}
impl ObservationFrames {
    fn len(&self) -> usize {
        match self {
            Self::Extracted(frames) => frames.len(),
            Self::Cached(frames) => frames.len(),
        }
    }
    fn timestamp(&self, index: usize) -> i64 {
        match self {
            Self::Extracted(frames) => frames.timestamp(index),
            Self::Cached(frames) => frames[index].timestamp_ms,
        }
    }
    fn frame(&self, index: usize) -> Result<FullFrame, AiError> {
        match self {
            Self::Extracted(frames) => frames.frame(index),
            Self::Cached(frames) => Ok(frames[index].clone()),
        }
    }
}
/// Matching OCR work has already extracted and compared these exact frames.
/// Reuse its validated outputs even after the bounded image cache prunes PNGs.
fn cached_frames(
    storage: &AppStorage,
    chunk: &ProcessingChunkRow,
    config: &VisionRequestConfig,
    fps: f64,
    adaptive: bool,
) -> Result<Option<Vec<FullFrame>>, AiError> {
    // Adaptive selection needs pixels and its own history. Shared extraction
    // and inference still reuse OCR work after the observation sampler runs.
    if adaptive {
        return Ok(None);
    }
    let Some(source) = chunk.track_id.as_deref() else {
        return Ok(None);
    };
    let start = chunk.start_offset_ms.unwrap_or(0);
    let end = chunk.end_offset_ms.unwrap_or(start);
    let fingerprint = shared_inference::fingerprint(config);
    storage.with_connection(|connection| {
        let cuts = storage::list_redactions(connection, &chunk.recording_id)?;
        let mut query = connection.prepare("SELECT frame_ms,end_ms,width,height,pixel_hash,source_frame_ref,context_start_ms FROM studio_visual_samples WHERE job_id=?1 AND source_id=?2 AND fingerprint=?3 AND frame_ms>=?4 AND frame_ms<?5 AND observation_json IS NOT NULL")?;
        let rows = query.query_map(rusqlite::params![chunk.job_id,source,fingerprint,start,end], |row| Ok((FullFrame {
            timestamp_ms: row.get(0)?, end_ms: row.get(1)?, width: row.get(2)?, height: row.get(3)?, pixel_hash: row.get(4)?, source_ref: row.get(5)?, path: PathBuf::new(),
        }, row.get::<_,i64>(6)?)))?.collect::<rusqlite::Result<Vec<_>>>()?;
        let by_time = rows.into_iter().map(|(frame, context)| (frame.timestamp_ms, (frame, context))).collect::<BTreeMap<_, _>>();
        let count = ((end - start) as f64 * fps / 1000.0).ceil() as usize;
        let mut frames = Vec::with_capacity(count);
        for index in 0..count {
            let time = start + (index as f64 * 1000.0 / fps).round() as i64;
            let Some((frame, context)) = by_time.get(&time) else { return Ok(None); };
            if crate::recording_privacy::range_redacted(&cuts, &[source.to_owned()], *context, frame.end_ms) { return Ok(None); }
            frames.push(frame.clone());
        }
        Ok(Some(frames))
    }).map_err(storage_error)
}
