use std::{env, path::PathBuf};

use serde::Serialize;
use serde_json::Value;
use tauri::{Manager, State};

use crate::recorder::{
    self, ffmpeg, ffmpeg::FfmpegPathSource, CaptureTimingDiagnostics, RecorderBackendHealth,
    RecorderFfmpegCapabilities, RecorderManager,
};
use crate::storage::{
    collect_database_diagnostics, get_latest_recording_id, list_recording_segments, AppStorage,
    DatabaseDiagnostics, RecordingSegmentRow,
};

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
    database: DatabaseDiagnostics,
    ffmpeg: FfmpegDiagnostics,
    latest_recording: LatestRecordingDiagnostics,
    recorder: RecorderBackendHealth,
    storage: StorageDiagnostics,
    system: SystemDiagnostics,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FfmpegDiagnostics {
    available: bool,
    capabilities: RecorderFfmpegCapabilities,
    error: Option<String>,
    path: Option<String>,
    source: FfmpegSource,
    version_line: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LatestRecordingDiagnostics {
    active_capture_timing: Option<CaptureTimingDiagnostics>,
    active_recording_id: Option<String>,
    active_segment_id: Option<String>,
    error: Option<String>,
    latest_segment: Option<LatestSegmentDiagnostics>,
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
    ffmpeg_exit_code: Option<i64>,
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

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum FfmpegSource {
    DaydreamFfmpegEnv,
    Path,
    Unconfigured,
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
    Ok(DaydreamDiagnostics {
        app: AppDiagnostics {
            identifier: app.config().identifier.to_string(),
            name: "Daydream".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
        database: collect_database_diagnostics(&storage),
        ffmpeg: collect_ffmpeg_diagnostics(),
        latest_recording: collect_latest_recording_diagnostics(&storage, &manager),
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

fn collect_ffmpeg_diagnostics() -> FfmpegDiagnostics {
    let health = ffmpeg::collect_ffmpeg_health();
    let source = ffmpeg::selected_ffmpeg_path_details()
        .map(|selected| source_from_ffmpeg_path_source(selected.source))
        .unwrap_or(FfmpegSource::Unconfigured);

    FfmpegDiagnostics {
        available: health.available,
        capabilities: health.capabilities,
        error: health.error,
        path: health.path,
        source,
        version_line: health.version_line,
    }
}

fn source_from_ffmpeg_path_source(source: FfmpegPathSource) -> FfmpegSource {
    match source {
        FfmpegPathSource::DaydreamFfmpegEnv => FfmpegSource::DaydreamFfmpegEnv,
        FfmpegPathSource::Path => FfmpegSource::Path,
    }
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
        recording_id: Some(recording_id),
        segment_count: segments.len(),
    }
}

fn latest_segment_diagnostics(segment: &RecordingSegmentRow) -> LatestSegmentDiagnostics {
    LatestSegmentDiagnostics {
        desktop_audio_stream_index: segment.desktop_audio_stream_index,
        duration_ms: segment.duration_ms,
        ended_at_ms: segment.ended_at_ms,
        error_code: segment.error_code.clone(),
        ffmpeg_exit_code: segment.ffmpeg_exit_code,
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

fn path_to_string(path: &PathBuf) -> String {
    path.to_string_lossy().to_string()
}

#[cfg(test)]
mod tests {
    use crate::recorder::ffmpeg::{parse_capabilities, FfmpegPathSource};

    use crate::storage::RecordingSegmentRow;

    use super::{latest_segment_diagnostics, source_from_ffmpeg_path_source};

    #[test]
    fn parses_ffmpeg_capability_summary() {
        let capabilities = parse_capabilities(
            " D  lavfi\n DE pulse\n",
            " DE rawvideo\n  E matroska\n",
            " V....D libx264\n A..... libopus\n",
        );

        assert!(capabilities.rawvideo_demuxer);
        assert!(capabilities.pulse_input);
        assert!(capabilities.matroska_muxer);
        assert!(capabilities.libx264_encoder);
        assert!(capabilities.libopus_encoder);
    }

    #[test]
    fn reports_path_sourced_ffmpeg_distinctly() {
        let source = source_from_ffmpeg_path_source(FfmpegPathSource::Path);

        assert_eq!(
            serde_json::to_value(source).expect("serialize ffmpeg source"),
            serde_json::json!("path"),
        );
    }

    #[test]
    fn reports_env_sourced_ffmpeg_distinctly() {
        let source = source_from_ffmpeg_path_source(FfmpegPathSource::DaydreamFfmpegEnv);

        assert_eq!(
            serde_json::to_value(source).expect("serialize ffmpeg source"),
            serde_json::json!("daydream-ffmpeg-env"),
        );
    }

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
    }
}
