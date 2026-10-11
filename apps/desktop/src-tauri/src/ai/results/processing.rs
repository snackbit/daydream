use super::super::*;

pub(crate) fn processing_state_for_recording(
    storage: &AppStorage,
    recording_id: &str,
) -> Result<RecordingAiProcessingState, AiError> {
    storage
        .with_connection(|connection| {
            let tx = connection.unchecked_transaction()?;
            let job_row = storage::get_latest_processing_job_for_recording(
                &tx,
                recording_id,
                AI_PROCESSING_JOB_KIND,
            )?;
            let chunk_rows = if let Some(job) = &job_row {
                storage::list_processing_chunks(&tx, &job.id)?
            } else {
                Vec::new()
            };
            let visual = storage::list_visual_frame_processing_states(&tx, recording_id)?;
            let transcript = super::transcript::transcript_from_connection(&tx, recording_id)?;
            let segments = storage::list_recording_segments(&tx, recording_id)?;
            let tracks = storage::list_recording_tracks(&tx, recording_id)?;
            let chunks = chunk_rows
                .iter()
                .cloned()
                .map(processing_chunk_from_row)
                .collect::<Vec<_>>();
            let history =
                super::transcription_history::transcription_history(&tx, recording_id, &chunks)?;
            let transcription_tracks = super::transcript::transcription_tracks_for_recording(
                recording_id,
                &segments,
                &tracks,
                &history,
                &transcript,
            );
            let transcription_progress = super::transcription_progress::source_progress(
                &tx,
                job_row.as_ref(),
                &chunk_rows,
                &tracks,
                &segments,
            )?;
            let (reprocessing_scopes, mut results_revision) =
                processing::reprocessing::progress(&tx, recording_id)?;
            // Publication changes must invalidate Studio and Recall reads even while
            // the containing legacy scheduling chunk is still running.
            if !transcription_progress.is_empty() {
                results_revision.push_str(&format!(
                    ":{:x}",
                    Sha1::digest(
                        serde_json::to_vec(&transcription_progress)
                            .map_err(|_| rusqlite::Error::InvalidQuery)?
                    )
                ));
            }
            let job = if let Some(row) = job_row {
                let (total, succeeded, average) =
                    storage::processing_job_chunk_progress(&tx, &row.id)?;
                let mut job = processing_job_from_row(row);
                if total > 0 {
                    job.progress = if succeeded == total { 1.0 } else { average };
                }
                Some(job)
            } else {
                None
            };
            tx.commit()?;
            Ok(RecordingAiProcessingState {
                transcription_progress,
                reprocessing_scopes,
                results_revision,
                chunks,
                job,
                recording_id: recording_id.into(),
                transcript,
                transcription_tracks,
                visual_frames: visual
                    .into_iter()
                    .map(visual_frame_processing_state_from_row)
                    .collect(),
            })
        })
        .map_err(storage_error)
}

pub(in crate::ai) fn processing_chunk_from_row(row: ProcessingChunkRow) -> ProcessingChunk {
    ProcessingChunk {
        attempt_count: row.attempt_count,
        created_at_ms: row.created_at_ms,
        end_offset_ms: row.end_offset_ms,
        error_code: row.error_code,
        id: row.id,
        job_id: row.job_id,
        progress: row.progress,
        recording_id: row.recording_id,
        result_code: row.result_code,
        segment_id: row.segment_id,
        source_kind: row.source_kind,
        start_offset_ms: row.start_offset_ms,
        status: row.status,
        track_id: row.track_id,
        updated_at_ms: row.updated_at_ms,
    }
}

pub(in crate::ai) fn visual_frame_processing_state_from_row(
    row: VisualFrameProcessingStateRow,
) -> VisualFrameProcessingState {
    VisualFrameProcessingState {
        chunk_id: row.chunk_id,
        created_at_ms: row.created_at_ms,
        error_code: row.error_code,
        frame_timestamp_ms: row.frame_timestamp_ms,
        id: row.id,
        job_id: row.job_id,
        output_count: row.output_count,
        progress: row.progress,
        recording_id: row.recording_id,
        region: VisualRegion {
            height: row.region_height,
            width: row.region_width,
            x: row.region_x,
            y: row.region_y,
        },
        segment_id: row.segment_id,
        source_frame_ref: row.source_frame_ref,
        source_kind: row.source_kind,
        status: row.status,
        updated_at_ms: row.updated_at_ms,
    }
}

pub(in crate::ai) fn processing_job_list_from_rows(
    rows: Vec<ProcessingJobRow>,
) -> ProcessingJobList {
    ProcessingJobList {
        jobs: rows.into_iter().map(processing_job_from_row).collect(),
    }
}

pub(crate) fn processing_job_from_row(row: ProcessingJobRow) -> ProcessingJob {
    ProcessingJob {
        created_at_ms: row.created_at_ms,
        error_code: row.error,
        id: row.id,
        kind: row.kind,
        progress: match row.status.as_str() {
            "succeeded" => 1.0,
            "running" => 0.5,
            _ => 0.0,
        },
        recording_id: row.recording_id,
        status: row.status,
        updated_at_ms: row.updated_at_ms,
    }
}

pub(in crate::ai) fn processing_job_from_row_with_storage(
    storage: &AppStorage,
    row: ProcessingJobRow,
) -> ProcessingJob {
    let progress = storage
        .with_connection(|connection| storage::processing_job_chunk_progress(connection, &row.id))
        .ok()
        .map(|(total, succeeded, average)| {
            if total == 0 {
                match row.status.as_str() {
                    "succeeded" => 1.0,
                    "running" => 0.5,
                    _ => 0.0,
                }
            } else if succeeded == total {
                1.0
            } else {
                average
            }
        })
        .unwrap_or(0.0);
    ProcessingJob {
        progress,
        ..processing_job_from_row(row)
    }
}

pub(crate) fn emit_processing_state<R: tauri::Runtime>(app: &AppHandle<R>, recording_id: &str) {
    // Invalidate the selected recording's read model instead of rebuilding and
    // broadcasting every transcript/word/frame to unrelated screens.
    let changed = crate::background_tasks::BackgroundTasksChanged {
        recording_id: Some(recording_id.into()),
        updated_at_ms: now_ms(),
    };
    let _ = app.emit(AI_PROCESSING_CHANGED_EVENT, &changed);
    crate::background_tasks::emit_changed(app, Some(recording_id));
    crate::recall::mutations::emit_changed(app, None);
}

pub(in crate::ai) fn processing_chunk_status_counts(chunks: &[ProcessingChunkRow]) -> String {
    let mut counts = BTreeMap::<String, usize>::new();
    for chunk in chunks {
        *counts.entry(chunk.status.clone()).or_default() += 1;
    }
    serde_json::to_string(&counts).unwrap_or_else(|_error| "{}".to_string())
}
