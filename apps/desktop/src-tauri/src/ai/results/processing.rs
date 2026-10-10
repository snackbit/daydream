use super::super::*;
use super::transcript::{transcript_for_recording, transcription_tracks_for_recording};

pub(crate) fn processing_state_for_recording(
    storage: &AppStorage,
    recording_id: &str,
) -> Result<RecordingAiProcessingState, AiError> {
    let job_row = storage
        .with_connection(|connection| {
            storage::get_latest_processing_job_for_recording(
                connection,
                recording_id,
                AI_PROCESSING_JOB_KIND,
            )
        })
        .map_err(storage_error)?;
    let chunk_rows = if let Some(job) = &job_row {
        storage
            .with_connection(|connection| storage::list_processing_chunks(connection, &job.id))
            .map_err(storage_error)?
    } else {
        Vec::new()
    };
    let visual_frame_rows = storage
        .with_connection(|connection| {
            storage::list_visual_frame_processing_states(connection, recording_id)
        })
        .map_err(storage_error)?;
    let transcript = transcript_for_recording(storage, recording_id)?;
    let recording_segments = storage
        .with_connection(|connection| storage::list_recording_segments(connection, recording_id))
        .map_err(storage_error)?;
    let recording_tracks = storage
        .with_connection(|connection| storage::list_recording_tracks(connection, recording_id))
        .map_err(storage_error)?;
    let chunks = chunk_rows
        .iter()
        .cloned()
        .map(processing_chunk_from_row)
        .collect::<Vec<_>>();
    let transcription_chunks =
        super::transcription_history::transcription_history(storage, recording_id, &chunks)?;
    let transcription_tracks = transcription_tracks_for_recording(
        recording_id,
        &recording_segments,
        &recording_tracks,
        &transcription_chunks,
        &transcript,
    );
    let (reprocessing_scopes, results_revision) = storage
        .with_connection(|connection| processing::reprocessing::progress(connection, recording_id))
        .map_err(storage_error)?;

    Ok(RecordingAiProcessingState {
        reprocessing_scopes,
        results_revision,
        chunks,
        job: job_row.map(|job| processing_job_from_row_with_storage(storage, job)),
        recording_id: recording_id.to_string(),
        transcript,
        transcription_tracks,
        visual_frames: visual_frame_rows
            .into_iter()
            .map(visual_frame_processing_state_from_row)
            .collect(),
    })
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
