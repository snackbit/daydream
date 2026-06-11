use std::{collections::BTreeMap, env, path::Path};

use serde::Serialize;
use serde_json::Value;
use tauri::{Manager, State};

use crate::recorder::{
    self, media_writer, CaptureTimingDiagnostics, RecorderBackendHealth, RecorderManager,
    RecorderMediaWriterCapabilities, RecorderPreviewStatus,
};
use crate::settings;
use crate::storage::{
    collect_database_diagnostics, get_latest_processing_job_for_recording, get_latest_recording_id,
    get_portal_restore_token, list_processing_chunks, list_recording_segments,
    list_recording_tracks, list_transcript_segments, AppStorage, DatabaseDiagnostics,
    ProcessingChunkRow, ProcessingJobRow, RecordingSegmentRow, TranscriptSegmentRow,
    LINUX_SCREENCAST_MONITOR_RESTORE_TOKEN_ID,
};
use crate::telemetry::{self, DebugLoggingDiagnostics};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppDiagnostics {
    identifier: String,
    name: String,
    version: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DaydreamDiagnostics {
    app: AppDiagnostics,
    ai_processing: AiProcessingDiagnostics,
    database: DatabaseDiagnostics,
    debug_logging: DebugLoggingDiagnostics,
    latest_recording: LatestRecordingDiagnostics,
    media_writer: MediaWriterDiagnostics,
    portal_restore_token: PortalRestoreTokenDiagnostics,
    recorder: RecorderBackendHealth,
    storage: StorageDiagnostics,
    system: SystemDiagnostics,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DebugLogBundle {
    diagnostics: DaydreamDiagnostics,
    log_tail_lines: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaWriterDiagnostics {
    available: bool,
    capabilities: RecorderMediaWriterCapabilities,
    error: Option<String>,
    version: Option<String>,
    writer: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiProcessingDiagnostics {
    chunk_count: usize,
    chunk_status_counts: BTreeMap<String, usize>,
    chunks: Vec<AiProcessingChunkDiagnostics>,
    error: Option<String>,
    latest_job: Option<AiProcessingJobDiagnostics>,
    recording_id: Option<String>,
    settings: Option<AiProcessingSettingsDiagnostics>,
    transcript_tracks: Vec<AiTranscriptTrackDiagnostics>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiProcessingSettingsDiagnostics {
    auto_start_after_recording: bool,
    max_parallel_chunks_per_job: u32,
    max_parallel_jobs: u32,
    transcription_backend: String,
    transcription_model: String,
    transcription_provider: String,
    word_timestamps_enabled: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiProcessingJobDiagnostics {
    created_at_ms: i64,
    error: Option<String>,
    id: String,
    kind: String,
    status: String,
    updated_at_ms: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiProcessingChunkDiagnostics {
    attempt_count: i64,
    end_offset_ms: Option<i64>,
    error_code: Option<String>,
    id: String,
    progress: f64,
    segment_id: Option<String>,
    source_kind: String,
    start_offset_ms: Option<i64>,
    status: String,
    track_id: Option<String>,
    track_kind: Option<String>,
    track_label: Option<String>,
    updated_at_ms: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiTranscriptTrackDiagnostics {
    first_start_offset_ms: Option<i64>,
    last_end_offset_ms: Option<i64>,
    segment_count: usize,
    source_role: String,
    text_char_count: usize,
    track_id: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PortalRestoreTokenDiagnostics {
    created_at_ms: Option<i64>,
    error: Option<String>,
    expected_id: String,
    last_used_at_ms: Option<i64>,
    portal: Option<String>,
    present: bool,
    token_length: Option<usize>,
    updated_at_ms: Option<i64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LatestRecordingDiagnostics {
    active_capture_timing: Option<CaptureTimingDiagnostics>,
    active_recording_id: Option<String>,
    active_segment_id: Option<String>,
    error: Option<String>,
    latest_segment: Option<LatestSegmentDiagnostics>,
    preview_status: Option<RecorderPreviewStatus>,
    recording_id: Option<String>,
    segment_count: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LatestSegmentDiagnostics {
    desktop_audio_stream_index: Option<i64>,
    duration_ms: Option<i64>,
    ended_at_ms: Option<i64>,
    error_code: Option<String>,
    writer_exit_code: Option<i64>,
    file_path: String,
    id: String,
    microphone_audio_stream_index: Option<i64>,
    probe: Option<Value>,
    recording_id: String,
    segment_index: i64,
    started_at_ms: i64,
    state: String,
    video_stream_index: Option<i64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageDiagnostics {
    app_data_dir: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemDiagnostics {
    arch: String,
    family: String,
    platform: String,
}

#[tauri::command]
pub async fn get_diagnostics(
    app: tauri::AppHandle,
    storage: State<'_, AppStorage>,
    manager: State<'_, RecorderManager>,
) -> Result<DaydreamDiagnostics, String> {
    collect_diagnostics(app, &storage, &manager).await
}

#[tauri::command]
pub async fn get_debug_log_bundle(
    app: tauri::AppHandle,
    storage: State<'_, AppStorage>,
    manager: State<'_, RecorderManager>,
) -> Result<DebugLogBundle, String> {
    Ok(DebugLogBundle {
        diagnostics: collect_diagnostics(app, &storage, &manager).await?,
        log_tail_lines: telemetry::read_debug_log_tail()?,
    })
}

async fn collect_diagnostics(
    app: tauri::AppHandle,
    storage: &AppStorage,
    manager: &RecorderManager,
) -> Result<DaydreamDiagnostics, String> {
    let latest_recording = collect_latest_recording_diagnostics(storage, manager);
    Ok(DaydreamDiagnostics {
        app: AppDiagnostics {
            identifier: app.config().identifier.to_string(),
            name: "Daydream".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
        ai_processing: collect_ai_processing_diagnostics(
            storage,
            latest_recording.recording_id.as_deref(),
        ),
        database: collect_database_diagnostics(storage),
        debug_logging: telemetry::diagnostics(),
        latest_recording,
        media_writer: collect_media_writer_diagnostics(),
        portal_restore_token: collect_portal_restore_token_diagnostics(storage),
        recorder: recorder::collect_backend_health().await,
        storage: StorageDiagnostics {
            app_data_dir: app
                .path()
                .app_data_dir()
                .ok()
                .map(|path| path_to_string(&path)),
        },
        system: SystemDiagnostics {
            arch: env::consts::ARCH.to_string(),
            family: env::consts::FAMILY.to_string(),
            platform: env::consts::OS.to_string(),
        },
    })
}

fn collect_media_writer_diagnostics() -> MediaWriterDiagnostics {
    let health = media_writer::collect_media_writer_health();

    MediaWriterDiagnostics {
        available: health.available,
        capabilities: health.capabilities,
        error: health.error,
        version: health.version,
        writer: health.writer,
    }
}

fn collect_ai_processing_diagnostics(
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
                transcription_backend: settings.ai.transcription.local_backend,
                transcription_model: settings.ai.transcription.local_model_id,
                transcription_provider: settings.ai.transcription.provider,
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
        let transcripts = list_transcript_segments(connection, recording_id)?;
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

fn transcript_track_diagnostics(
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

fn collect_latest_recording_diagnostics(
    storage: &AppStorage,
    manager: &RecorderManager,
) -> LatestRecordingDiagnostics {
    let status = manager.status().ok();
    let active_recording_id = status
        .as_ref()
        .and_then(|status| status.active_recording_id.clone());
    let active_segment_id = status
        .as_ref()
        .and_then(|status| status.active_segment_id.clone());
    let active_capture_timing = manager.active_capture_timing().ok().flatten();
    let preview_status = manager.preview_status().ok();

    let recording_id = match active_recording_id.clone() {
        Some(recording_id) => Some(recording_id),
        None => match storage.with_connection(get_latest_recording_id) {
            Ok(recording_id) => recording_id,
            Err(error) => {
                return LatestRecordingDiagnostics {
                    active_capture_timing,
                    active_recording_id,
                    active_segment_id,
                    error: Some(error.to_string()),
                    latest_segment: None,
                    preview_status,
                    recording_id: None,
                    segment_count: 0,
                };
            }
        },
    };

    let Some(recording_id) = recording_id else {
        return LatestRecordingDiagnostics {
            active_capture_timing,
            active_recording_id,
            active_segment_id,
            error: None,
            latest_segment: None,
            preview_status,
            recording_id: None,
            segment_count: 0,
        };
    };

    let segments = match storage
        .with_connection(|connection| list_recording_segments(connection, &recording_id))
    {
        Ok(segments) => segments,
        Err(error) => {
            return LatestRecordingDiagnostics {
                active_capture_timing,
                active_recording_id,
                active_segment_id,
                error: Some(error.to_string()),
                latest_segment: None,
                preview_status,
                recording_id: Some(recording_id),
                segment_count: 0,
            };
        }
    };

    LatestRecordingDiagnostics {
        active_capture_timing,
        active_recording_id,
        active_segment_id,
        error: None,
        latest_segment: segments.last().map(latest_segment_diagnostics),
        preview_status,
        recording_id: Some(recording_id),
        segment_count: segments.len(),
    }
}

fn collect_portal_restore_token_diagnostics(storage: &AppStorage) -> PortalRestoreTokenDiagnostics {
    match storage.with_connection(|connection| {
        get_portal_restore_token(connection, LINUX_SCREENCAST_MONITOR_RESTORE_TOKEN_ID)
    }) {
        Ok(Some(row)) => PortalRestoreTokenDiagnostics {
            created_at_ms: Some(row.created_at_ms),
            error: None,
            expected_id: LINUX_SCREENCAST_MONITOR_RESTORE_TOKEN_ID.to_string(),
            last_used_at_ms: row.last_used_at_ms,
            portal: Some(row.portal),
            present: true,
            token_length: Some(row.token.len()),
            updated_at_ms: Some(row.updated_at_ms),
        },
        Ok(None) => PortalRestoreTokenDiagnostics {
            created_at_ms: None,
            error: None,
            expected_id: LINUX_SCREENCAST_MONITOR_RESTORE_TOKEN_ID.to_string(),
            last_used_at_ms: None,
            portal: None,
            present: false,
            token_length: None,
            updated_at_ms: None,
        },
        Err(error) => PortalRestoreTokenDiagnostics {
            created_at_ms: None,
            error: Some(error.to_string()),
            expected_id: LINUX_SCREENCAST_MONITOR_RESTORE_TOKEN_ID.to_string(),
            last_used_at_ms: None,
            portal: None,
            present: false,
            token_length: None,
            updated_at_ms: None,
        },
    }
}

fn latest_segment_diagnostics(segment: &RecordingSegmentRow) -> LatestSegmentDiagnostics {
    LatestSegmentDiagnostics {
        desktop_audio_stream_index: segment.desktop_audio_stream_index,
        duration_ms: segment.duration_ms,
        ended_at_ms: segment.ended_at_ms,
        error_code: segment.error_code.clone(),
        writer_exit_code: segment.ffmpeg_exit_code,
        file_path: segment.file_path.clone(),
        id: segment.id.clone(),
        microphone_audio_stream_index: segment.microphone_audio_stream_index,
        probe: segment
            .probe_json
            .as_deref()
            .and_then(|probe_json| serde_json::from_str(probe_json).ok()),
        recording_id: segment.recording_id.clone(),
        segment_index: segment.segment_index,
        started_at_ms: segment.started_at_ms,
        state: segment.state.clone(),
        video_stream_index: segment.video_stream_index,
    }
}

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

#[cfg(test)]
mod tests {
    use crate::storage::{RecordingSegmentRow, TranscriptSegmentRow};

    use super::{latest_segment_diagnostics, transcript_track_diagnostics};

    #[test]
    fn parses_latest_segment_probe_metadata() {
        let segment = RecordingSegmentRow {
            desktop_audio_stream_index: Some(1),
            duration_ms: Some(1200),
            ended_at_ms: Some(2200),
            error_code: None,
            ffmpeg_exit_code: Some(0),
            file_path: "/tmp/daydream/segment-0.mkv".to_string(),
            id: "segment-1".to_string(),
            microphone_audio_stream_index: Some(2),
            probe_json: Some(
                serde_json::json!({
                    "available": true,
                    "durationMs": 1200,
                    "streams": [
                        {
                            "index": 0,
                            "kind": "video",
                            "codecName": "h264"
                        }
                    ]
                })
                .to_string(),
            ),
            recording_id: "recording-1".to_string(),
            segment_index: 0,
            started_at_ms: 1000,
            state: "finalized".to_string(),
            video_stream_index: Some(0),
        };

        let diagnostics = latest_segment_diagnostics(&segment);

        assert_eq!(
            diagnostics
                .probe
                .as_ref()
                .and_then(|probe| probe.get("available"))
                .and_then(|available| available.as_bool()),
            Some(true)
        );
        assert_eq!(
            diagnostics
                .probe
                .as_ref()
                .and_then(|probe| probe.get("durationMs"))
                .and_then(|duration| duration.as_u64()),
            Some(1200)
        );
        assert_eq!(diagnostics.writer_exit_code, Some(0));
    }

    #[test]
    fn transcript_track_diagnostics_omit_private_text() {
        let diagnostics = transcript_track_diagnostics(vec![TranscriptSegmentRow {
            chunk_id: "chunk-1".to_string(),
            confidence: Some(0.9),
            created_at_ms: 1,
            end_offset_ms: 900,
            id: "transcript-1".to_string(),
            job_id: "job-1".to_string(),
            model: Some("large-v3-turbo".to_string()),
            provider: "local-whisper-cpp".to_string(),
            recording_id: "recording-1".to_string(),
            segment_id: "segment-1".to_string(),
            source_role: "microphone".to_string(),
            start_offset_ms: 100,
            text: "private transcript text".to_string(),
            track_id: "track-1".to_string(),
        }]);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].segment_count, 1);
        assert_eq!(diagnostics[0].text_char_count, 23);
        assert_eq!(diagnostics[0].first_start_offset_ms, Some(100));
        assert_eq!(diagnostics[0].last_end_offset_ms, Some(900));
    }
}
