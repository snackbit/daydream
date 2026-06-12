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
    let transcription_tracks = transcription_tracks_for_recording(
        recording_id,
        &recording_segments,
        &recording_tracks,
        &chunks,
        &transcript,
    );

    Ok(RecordingAiProcessingState {
        chunks,
        job: job_row.map(|job| processing_job_from_row_with_storage(storage, job)),
        recording_id: recording_id.to_string(),
        transcript,
        transcription_tracks,
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

pub(in crate::ai) fn processing_job_list_from_rows(
    rows: Vec<ProcessingJobRow>,
) -> ProcessingJobList {
    ProcessingJobList {
        jobs: rows.into_iter().map(processing_job_from_row).collect(),
    }
}

fn processing_job_from_row(row: ProcessingJobRow) -> ProcessingJob {
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

pub(in crate::ai) fn emit_processing_state<R: tauri::Runtime>(
    app: &AppHandle<R>,
    recording_id: &str,
) {
    let storage = app.state::<AppStorage>();
    if let Ok(state) = processing_state_for_recording(&storage, recording_id) {
        tracing::info!(
            recording_id = %recording_id,
            job_id = state.job.as_ref().map(|job| job.id.as_str()).unwrap_or(""),
            job_status = state.job.as_ref().map(|job| job.status.as_str()).unwrap_or("none"),
            chunk_status_counts = %processing_state_chunk_status_counts(&state.chunks),
            transcript_track_count = state.transcript.tracks.len(),
            "ai_processing_state_emitted"
        );
        let _ = app.emit(AI_PROCESSING_CHANGED_EVENT, state);
    }
}

pub(in crate::ai) fn processing_chunk_status_counts(chunks: &[ProcessingChunkRow]) -> String {
    let mut counts = BTreeMap::<String, usize>::new();
    for chunk in chunks {
        *counts.entry(chunk.status.clone()).or_default() += 1;
    }
    serde_json::to_string(&counts).unwrap_or_else(|_error| "{}".to_string())
}

pub(in crate::ai) fn processing_state_chunk_status_counts(chunks: &[ProcessingChunk]) -> String {
    let mut counts = BTreeMap::<String, usize>::new();
    for chunk in chunks {
        *counts.entry(chunk.status.clone()).or_default() += 1;
    }
    serde_json::to_string(&counts).unwrap_or_else(|_error| "{}".to_string())
}
