use std::collections::BTreeMap;

use super::types::{
    AiProcessingChunkDiagnostics, AiProcessingDiagnostics, AiProcessingJobDiagnostics,
    AiProcessingSettingsDiagnostics, AiTranscriptTrackDiagnostics,
};
use crate::settings;
use crate::storage::{
    get_latest_processing_job_for_recording, list_processing_chunks, list_recording_tracks,
    list_redactions, list_transcript_segments, redaction_applies_to_track_range, AppStorage,
    ProcessingChunkRow, ProcessingJobRow, TranscriptSegmentRow,
};

pub(super) fn collect_ai_processing_diagnostics(
    storage: &AppStorage,
    recording_id: Option<&str>,
) -> AiProcessingDiagnostics {
    let settings =
        settings::load_app_settings(storage)
            .ok()
            .map(|settings| AiProcessingSettingsDiagnostics {
                auto_start_after_recording: settings.ai.processing.auto_start_after_recording,
                max_parallel_chunks_per_job: settings.ai.processing.max_parallel_chunks_per_job,
                max_parallel_jobs: settings.ai.processing.max_parallel_jobs,
                transcription_max_initial_timestamp_seconds: settings
                    .ai
                    .transcription
                    .max_initial_timestamp_seconds,
                transcription_merge_gap_ms: settings.ai.transcription.merge_gap_ms,
                transcription_backend: settings.ai.transcription.local_backend,
                transcription_language: settings.ai.transcription.language,
                transcription_min_speech_ms: settings.ai.transcription.min_speech_ms,
                transcription_model: settings.ai.transcription.local_model_id,
                transcription_provider: settings.ai.transcription.provider,
                transcription_silence_gate_enabled: settings.ai.transcription.silence_gate_enabled,
                transcription_silence_threshold_db: settings.ai.transcription.silence_threshold_db,
                word_timestamps_enabled: settings.ai.transcription.word_timestamps_enabled,
            });

    let Some(recording_id) = recording_id else {
        return AiProcessingDiagnostics {
            chunk_count: 0,
            chunk_status_counts: BTreeMap::new(),
            chunks: Vec::new(),
            error: None,
            latest_job: None,
            recording_id: None,
            settings,
            transcript_tracks: Vec::new(),
        };
    };

    let result = storage.with_connection(|connection| {
        let tracks = list_recording_tracks(connection, recording_id)?;
        let tracks_by_id = tracks
            .into_iter()
            .map(|track| (track.id.clone(), (track.kind, track.label)))
            .collect::<BTreeMap<_, _>>();
        let latest_job =
            get_latest_processing_job_for_recording(connection, recording_id, "ai-processing")?;
        let chunks = match &latest_job {
            Some(job) => list_processing_chunks(connection, &job.id)?,
            None => Vec::new(),
        };
        let redactions = list_redactions(connection, recording_id)?;
        let transcripts = list_transcript_segments(connection, recording_id)?
            .into_iter()
            .filter(|transcript| {
                !redactions.iter().any(|redaction| {
                    redaction_applies_to_track_range(
                        redaction,
                        &transcript.track_id,
                        transcript.start_offset_ms,
                        transcript.end_offset_ms,
                    )
                })
            })
            .collect::<Vec<_>>();
        Ok((tracks_by_id, latest_job, chunks, transcripts))
    });

    let Ok((tracks_by_id, latest_job, chunks, transcripts)) = result else {
        return AiProcessingDiagnostics {
            chunk_count: 0,
            chunk_status_counts: BTreeMap::new(),
            chunks: Vec::new(),
            error: result.err().map(|error| error.to_string()),
            latest_job: None,
            recording_id: Some(recording_id.to_string()),
            settings,
            transcript_tracks: Vec::new(),
        };
    };

    let chunk_status_counts = chunk_status_counts(&chunks);
    AiProcessingDiagnostics {
        chunk_count: chunks.len(),
        chunk_status_counts,
        chunks: chunks
            .into_iter()
            .map(|chunk| chunk_diagnostics(chunk, &tracks_by_id))
            .collect(),
        error: None,
        latest_job: latest_job.map(job_diagnostics),
        recording_id: Some(recording_id.to_string()),
        settings,
        transcript_tracks: transcript_track_diagnostics(transcripts),
    }
}

fn job_diagnostics(job: ProcessingJobRow) -> AiProcessingJobDiagnostics {
    AiProcessingJobDiagnostics {
        created_at_ms: job.created_at_ms,
        error: job.error,
        id: job.id,
        kind: job.kind,
        status: job.status,
        updated_at_ms: job.updated_at_ms,
    }
}

fn chunk_diagnostics(
    chunk: ProcessingChunkRow,
    tracks_by_id: &BTreeMap<String, (String, String)>,
) -> AiProcessingChunkDiagnostics {
    let track = chunk
        .track_id
        .as_ref()
        .and_then(|track_id| tracks_by_id.get(track_id));

    AiProcessingChunkDiagnostics {
        attempt_count: chunk.attempt_count,
        end_offset_ms: chunk.end_offset_ms,
        error_code: chunk.error_code,
        id: chunk.id,
        progress: chunk.progress,
        result_code: chunk.result_code,
        segment_id: chunk.segment_id,
        source_kind: chunk.source_kind,
        start_offset_ms: chunk.start_offset_ms,
        status: chunk.status,
        track_id: chunk.track_id,
        track_kind: track.map(|(kind, _label)| kind.clone()),
        track_label: track.map(|(_kind, label)| label.clone()),
        updated_at_ms: chunk.updated_at_ms,
    }
}

fn chunk_status_counts(chunks: &[ProcessingChunkRow]) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for chunk in chunks {
        *counts.entry(chunk.status.clone()).or_default() += 1;
    }
    counts
}

pub(super) fn transcript_track_diagnostics(
    transcripts: Vec<TranscriptSegmentRow>,
) -> Vec<AiTranscriptTrackDiagnostics> {
    let mut grouped = BTreeMap::<(String, String), Vec<TranscriptSegmentRow>>::new();
    for segment in transcripts {
        grouped
            .entry((segment.track_id.clone(), segment.source_role.clone()))
            .or_default()
            .push(segment);
    }

    grouped
        .into_iter()
        .map(
            |((track_id, source_role), segments)| AiTranscriptTrackDiagnostics {
                first_start_offset_ms: segments.iter().map(|segment| segment.start_offset_ms).min(),
                last_end_offset_ms: segments.iter().map(|segment| segment.end_offset_ms).max(),
                segment_count: segments.len(),
                source_role,
                text_char_count: segments
                    .iter()
                    .map(|segment| segment.text.chars().count())
                    .sum(),
                track_id,
            },
        )
        .collect()
}
