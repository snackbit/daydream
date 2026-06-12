use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::{Manager, State};
use uuid::Uuid;

use crate::{
    ai, ffmpeg,
    playback_server::PlaybackServer,
    settings,
    storage::{
        self, AppStorage, ProcessingChunkRow, ProcessingJobRow, RecordingRow, RecordingSegmentRow,
        RecordingTrackRow, RedactionRow, TranscriptSegmentRow, TranscriptWordRow,
    },
};

const RECORDING_TITLE_MAX_CHARS: usize = 120;

mod waveforms;

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum LibraryRecordingAvailability {
    Available,
    Incomplete,
    Locked,
    MissingFiles,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryRecordingSummary {
    availability: LibraryRecordingAvailability,
    duration_ms: Option<i64>,
    ended_at_ms: Option<i64>,
    id: String,
    missing_reasons: Vec<String>,
    playable_segment_count: usize,
    segment_count: usize,
    size_bytes: u64,
    started_at_ms: Option<i64>,
    status: String,
    storage_dir: String,
    thumbnail_path: Option<String>,
    title: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryRecordingDetail {
    availability: LibraryRecordingAvailability,
    duration_ms: Option<i64>,
    ended_at_ms: Option<i64>,
    id: String,
    missing_reasons: Vec<String>,
    playable_segment_count: usize,
    redactions: Vec<LibraryRecordingRedaction>,
    segments: Vec<LibraryRecordingSegment>,
    size_bytes: u64,
    started_at_ms: Option<i64>,
    status: String,
    storage_dir: String,
    thumbnail_path: Option<String>,
    title: Option<String>,
    tracks: Vec<LibraryRecordingTrack>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryRecordingSegment {
    desktop_audio_stream_index: Option<i64>,
    duration_ms: Option<i64>,
    ended_at_ms: Option<i64>,
    file_exists: bool,
    file_path: String,
    id: String,
    microphone_audio_stream_index: Option<i64>,
    pause_after_ms: i64,
    playback_offset_ms: i64,
    segment_index: i64,
    started_at_ms: i64,
    state: String,
    video_stream_index: Option<i64>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryRecordingTrack {
    codec: Option<String>,
    description: String,
    id: String,
    kind: LibraryTrackKind,
    label: String,
    stream_index: Option<i64>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryRecordingRedaction {
    created_at_ms: i64,
    end_offset_ms: i64,
    id: String,
    reason: Option<String>,
    recording_id: String,
    start_offset_ms: i64,
    track_ids: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum LibraryTrackKind {
    Video,
    DesktopAudio,
    Microphone,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteLibraryRecordingInput {
    recording_id: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteLibraryRecordingsInput {
    recording_ids: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenameLibraryRecordingInput {
    recording_id: String,
    title: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateLibraryRedactionRangeInput {
    end_offset_ms: i64,
    start_offset_ms: i64,
    track_ids: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateLibraryRedactionsInput {
    recording_id: String,
    ranges: Vec<CreateLibraryRedactionRangeInput>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteLibraryRedactionInput {
    recording_id: String,
    redaction_id: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnredactLibraryRedactionsInput {
    recording_id: String,
    ranges: Vec<CreateLibraryRedactionRangeInput>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryWaveformRequest {
    recording_id: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryWaveformTileRequest {
    recording_id: String,
    start_timeline_ms: i64,
    end_timeline_ms: i64,
    viewport_width_px: f64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryPlaybackAssetRequest {
    recording_id: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloseLibraryPlaybackSessionRequest {
    session_id: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryRecordingThumbnail {
    error: Option<String>,
    path: Option<String>,
    recording_id: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryRecordingThumbnailRequest {
    recording_id: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryRecordingDebugInput {
    recording_id: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryRecordingDebugData {
    detail: LibraryRecordingDetail,
    processing_state: ai::RecordingAiProcessingState,
    raw: Value,
    recording_id: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryPlaybackAssets {
    pub error: Option<String>,
    pub recording_id: String,
    pub segments: Vec<LibraryPlaybackSegment>,
    pub session_id: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryPlaybackSegment {
    pub audio: Vec<LibraryPlaybackAudioAsset>,
    pub segment_id: String,
    pub thumbnails: Vec<LibraryPlaybackThumbnail>,
    pub video_url: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryPlaybackAudioAsset {
    pub kind: LibraryTrackKind,
    pub stream_index: i64,
    pub track_id: String,
    pub url: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryPlaybackThumbnail {
    pub offset_ms: i64,
    pub url: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryWaveforms {
    error: Option<String>,
    recording_id: String,
    tracks: Vec<LibraryWaveformTrack>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryWaveformTrack {
    kind: LibraryTrackKind,
    segments: Vec<LibraryWaveformSegment>,
    track_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryWaveformSegment {
    generated_at_ms: i64,
    max_amplitude: f64,
    samples: Vec<f64>,
    segment_id: String,
    stream_index: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LibraryWaveformTiles {
    bin_ms: i64,
    error: Option<String>,
    recording_id: String,
    tracks: Vec<LibraryWaveformTileTrack>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LibraryWaveformTileTrack {
    kind: LibraryTrackKind,
    scale_amplitude: f64,
    tiles: Vec<LibraryWaveformTile>,
    track_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LibraryWaveformTile {
    bin_ms: i64,
    duration_ms: i64,
    generated_at_ms: i64,
    peak_samples: Vec<f64>,
    rms_samples: Vec<f64>,
    segment_id: String,
    stream_index: i64,
    timeline_start_ms: i64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SegmentProbe {
    audio: Vec<SegmentAudioProbe>,
    video: Option<SegmentVideoProbe>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SegmentAudioProbe {
    codec_name: Option<String>,
    index: i64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SegmentVideoProbe {
    codec_name: Option<String>,
    index: i64,
}

#[tauri::command(rename = "listLibraryRecordings")]
pub fn list_library_recordings(
    storage: State<'_, AppStorage>,
) -> Result<Vec<LibraryRecordingSummary>, String> {
    let recordings = storage
        .with_connection(storage::list_recordings)
        .map_err(|error| error.to_string())?;

    recordings
        .into_iter()
        .map(|recording| {
            let segments = storage
                .with_connection(|connection| {
                    storage::list_recording_segments(connection, &recording.id)
                })
                .map_err(|error| error.to_string())?;
            Ok(summary_from_rows(&recording, &segments))
        })
        .collect()
}

#[tauri::command(rename = "getLibraryRecording")]
pub fn get_library_recording(
    recording_id: String,
    storage: State<'_, AppStorage>,
) -> Result<LibraryRecordingDetail, String> {
    let recording = storage
        .with_connection(|connection| storage::get_recording(connection, &recording_id))
        .map_err(|error| error.to_string())?;
    let segments = storage
        .with_connection(|connection| storage::list_recording_segments(connection, &recording_id))
        .map_err(|error| error.to_string())?;
    let redactions = storage
        .with_connection(|connection| storage::list_redactions(connection, &recording_id))
        .map_err(|error| error.to_string())?;

    Ok(detail_from_rows(&recording, &segments, &redactions))
}

#[tauri::command(rename = "getLibraryRecordingDebug")]
pub fn get_library_recording_debug(
    input: LibraryRecordingDebugInput,
    storage: State<'_, AppStorage>,
) -> Result<LibraryRecordingDebugData, String> {
    ensure_debug_tools_enabled(storage.inner())?;

    let recording = storage
        .with_connection(|connection| storage::get_recording(connection, &input.recording_id))
        .map_err(|error| error.to_string())?;
    let segments = storage
        .with_connection(|connection| {
            storage::list_recording_segments(connection, &input.recording_id)
        })
        .map_err(|error| error.to_string())?;
    let tracks = storage
        .with_connection(|connection| {
            storage::list_recording_tracks(connection, &input.recording_id)
        })
        .map_err(|error| error.to_string())?;
    let jobs = storage
        .with_connection(|connection| {
            storage::list_processing_jobs(connection, Some(&input.recording_id))
        })
        .map_err(|error| error.to_string())?;
    let chunks = jobs
        .iter()
        .map(|job| {
            storage
                .with_connection(|connection| storage::list_processing_chunks(connection, &job.id))
                .map_err(|error| error.to_string())
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    let transcript_segments =
        ai::visible_transcript_segments_for_recording(storage.inner(), &input.recording_id)
            .map_err(|error| format!("{}: {}", error.code, error.message))?;
    let transcript_words = transcript_segments
        .iter()
        .map(|segment| {
            storage
                .with_connection(|connection| {
                    storage::list_transcript_words(connection, &segment.id)
                })
                .map_err(|error| error.to_string())
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    let activity_intervals = storage
        .with_connection(|connection| {
            storage::list_activity_intervals(connection, &input.recording_id)
        })
        .map_err(|error| error.to_string())?;
    let redactions = storage
        .with_connection(|connection| storage::list_redactions(connection, &input.recording_id))
        .map_err(|error| error.to_string())?;
    let processing_state = ai::processing_state_for_recording(&storage, &input.recording_id)
        .map_err(|error| format!("{}: {}", error.code, error.message))?;
    let detail = detail_from_rows(&recording, &segments, &redactions);

    Ok(LibraryRecordingDebugData {
        raw: json!({
            "recording": recording_row_json(&recording),
            "segments": segments.iter().map(recording_segment_row_json).collect::<Vec<_>>(),
            "tracks": tracks.iter().map(recording_track_row_json).collect::<Vec<_>>(),
            "processingJobs": jobs.iter().map(processing_job_row_json).collect::<Vec<_>>(),
            "processingChunks": chunks.iter().map(processing_chunk_row_json).collect::<Vec<_>>(),
            "transcriptSegments": transcript_segments.iter().map(transcript_segment_row_json).collect::<Vec<_>>(),
            "transcriptWords": transcript_words.iter().map(transcript_word_row_json).collect::<Vec<_>>(),
            "redactions": redactions.iter().map(redaction_row_json).collect::<Vec<_>>(),
            "activityIntervals": activity_intervals.iter().map(|interval| json!({
                "chunkId": interval.chunk_id,
                "endOffsetMs": interval.end_offset_ms,
                "id": interval.id,
                "jobId": interval.job_id,
                "recordingId": interval.recording_id,
                "regionsJson": interval.regions_json,
                "score": interval.score,
                "sourceKind": interval.source_kind,
                "startOffsetMs": interval.start_offset_ms,
                "state": interval.state,
            })).collect::<Vec<_>>(),
        }),
        detail,
        processing_state,
        recording_id: input.recording_id,
    })
}

fn ensure_debug_tools_enabled(storage: &AppStorage) -> Result<(), String> {
    let app_settings = settings::load_app_settings(storage).map_err(|error| error.message)?;
    if !app_settings.debug.enabled {
        return Err("Debug tools are disabled.".to_string());
    }

    Ok(())
}

#[tauri::command(rename = "deleteLibraryRecording")]
pub fn delete_library_recording(
    app: tauri::AppHandle,
    input: DeleteLibraryRecordingInput,
    storage: State<'_, AppStorage>,
) -> Result<(), String> {
    let recordings_root = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?
        .join("recordings");
    let recording = storage
        .with_connection(|connection| storage::get_recording(connection, &input.recording_id))
        .map_err(|error| error.to_string())?;

    ensure_recording_is_deletable(&recording)?;
    delete_recording_row(&storage, &recordings_root, &recording)
}

#[tauri::command(rename = "deleteLibraryRecordings")]
pub fn delete_library_recordings(
    app: tauri::AppHandle,
    input: DeleteLibraryRecordingsInput,
    storage: State<'_, AppStorage>,
) -> Result<(), String> {
    let recordings_root = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?
        .join("recordings");
    let recording_ids = unique_recording_ids(input.recording_ids);
    let recordings = recording_ids
        .iter()
        .map(|recording_id| {
            storage
                .with_connection(|connection| storage::get_recording(connection, recording_id))
                .map_err(|error| error.to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;

    for recording in &recordings {
        ensure_recording_is_deletable(recording)?;
    }

    for recording in recordings {
        delete_recording_row(&storage, &recordings_root, &recording)?;
    }

    Ok(())
}

#[tauri::command(rename = "renameLibraryRecording")]
pub fn rename_library_recording(
    input: RenameLibraryRecordingInput,
    storage: State<'_, AppStorage>,
) -> Result<LibraryRecordingDetail, String> {
    let title = normalize_recording_title(input.title)?;
    let recording = storage
        .with_connection(|connection| {
            storage::rename_recording(connection, &input.recording_id, title.as_deref())
        })
        .map_err(|error| error.to_string())?;
    let segments = storage
        .with_connection(|connection| storage::list_recording_segments(connection, &recording.id))
        .map_err(|error| error.to_string())?;
    let redactions = storage
        .with_connection(|connection| storage::list_redactions(connection, &recording.id))
        .map_err(|error| error.to_string())?;

    Ok(detail_from_rows(&recording, &segments, &redactions))
}

#[tauri::command(rename = "createLibraryRedactions")]
pub fn create_library_redactions(
    input: CreateLibraryRedactionsInput,
    storage: State<'_, AppStorage>,
) -> Result<LibraryRecordingDetail, String> {
    let recording = storage
        .with_connection(|connection| storage::get_recording(connection, &input.recording_id))
        .map_err(|error| error.to_string())?;
    ensure_recording_allows_redaction(&recording)?;
    let segments = storage
        .with_connection(|connection| {
            storage::list_recording_segments(connection, &input.recording_id)
        })
        .map_err(|error| error.to_string())?;
    let tracks = tracks_from_segments(&segments);
    let duration_ms = timeline_duration_ms(&segments);
    let existing_redactions = storage
        .with_connection(|connection| storage::list_redactions(connection, &recording.id))
        .map_err(|error| error.to_string())?;
    let inputs = build_normalized_redaction_inputs(
        &recording.id,
        &input.ranges,
        &existing_redactions,
        &tracks,
        duration_ms,
    )?;

    storage
        .with_connection_mut(|connection| {
            let transaction = connection.transaction()?;
            storage::delete_redactions_for_recording(&transaction, &recording.id)?;
            for input in &inputs {
                storage::create_redaction(&transaction, input)?;
            }

            transaction.commit()
        })
        .map_err(|error| error.to_string())?;

    detail_for_recording(storage.inner(), &recording.id)
}

#[tauri::command(rename = "deleteLibraryRedaction")]
pub fn delete_library_redaction(
    input: DeleteLibraryRedactionInput,
    storage: State<'_, AppStorage>,
) -> Result<LibraryRecordingDetail, String> {
    let recording = storage
        .with_connection(|connection| storage::get_recording(connection, &input.recording_id))
        .map_err(|error| error.to_string())?;
    ensure_recording_allows_redaction(&recording)?;
    let deleted = storage
        .with_connection(|connection| {
            storage::delete_redaction(connection, &input.recording_id, &input.redaction_id)
        })
        .map_err(|error| error.to_string())?;
    if !deleted {
        return Err("Redaction was not found.".to_string());
    }

    detail_for_recording(storage.inner(), &recording.id)
}

#[tauri::command(rename = "unredactLibraryRedactions")]
pub fn unredact_library_redactions(
    input: UnredactLibraryRedactionsInput,
    storage: State<'_, AppStorage>,
) -> Result<LibraryRecordingDetail, String> {
    let recording = storage
        .with_connection(|connection| storage::get_recording(connection, &input.recording_id))
        .map_err(|error| error.to_string())?;
    ensure_recording_allows_redaction(&recording)?;
    let segments = storage
        .with_connection(|connection| {
            storage::list_recording_segments(connection, &input.recording_id)
        })
        .map_err(|error| error.to_string())?;
    let tracks = tracks_from_segments(&segments);
    let duration_ms = timeline_duration_ms(&segments);
    let existing_redactions = storage
        .with_connection(|connection| storage::list_redactions(connection, &recording.id))
        .map_err(|error| error.to_string())?;
    let inputs = build_unredacted_redaction_inputs(
        &recording.id,
        &input.ranges,
        &existing_redactions,
        &tracks,
        duration_ms,
    )?;

    storage
        .with_connection_mut(|connection| {
            let transaction = connection.transaction()?;
            storage::delete_redactions_for_recording(&transaction, &recording.id)?;
            for input in &inputs {
                storage::create_redaction(&transaction, input)?;
            }

            transaction.commit()
        })
        .map_err(|error| error.to_string())?;

    detail_for_recording(storage.inner(), &recording.id)
}

#[tauri::command(rename = "getRecordingWaveforms")]
pub fn get_recording_waveforms(
    input: LibraryWaveformRequest,
    storage: State<'_, AppStorage>,
) -> Result<LibraryWaveforms, String> {
    let recording = storage
        .with_connection(|connection| storage::get_recording(connection, &input.recording_id))
        .map_err(|error| error.to_string())?;
    let segments = storage
        .with_connection(|connection| {
            storage::list_recording_segments(connection, &input.recording_id)
        })
        .map_err(|error| error.to_string())?;

    Ok(waveforms::waveforms_for_recording(&recording, &segments))
}

#[tauri::command(rename = "getRecordingWaveformTiles")]
pub fn get_recording_waveform_tiles(
    input: LibraryWaveformTileRequest,
    storage: State<'_, AppStorage>,
) -> Result<LibraryWaveformTiles, String> {
    let recording = storage
        .with_connection(|connection| storage::get_recording(connection, &input.recording_id))
        .map_err(|error| error.to_string())?;
    let segments = storage
        .with_connection(|connection| {
            storage::list_recording_segments(connection, &input.recording_id)
        })
        .map_err(|error| error.to_string())?;

    Ok(waveforms::waveform_tiles_for_recording(
        &recording, &segments, &input,
    ))
}

#[tauri::command(rename = "createLibraryPlaybackSession")]
pub fn create_library_playback_session(
    input: LibraryPlaybackAssetRequest,
    storage: State<'_, AppStorage>,
    playback_server: State<'_, PlaybackServer>,
) -> Result<LibraryPlaybackAssets, String> {
    let recording = storage
        .with_connection(|connection| storage::get_recording(connection, &input.recording_id))
        .map_err(|error| error.to_string())?;
    let segments = storage
        .with_connection(|connection| {
            storage::list_recording_segments(connection, &input.recording_id)
        })
        .map_err(|error| error.to_string())?;
    let detail = detail_from_rows(&recording, &segments, &[]);
    if detail.availability != LibraryRecordingAvailability::Available {
        return Ok(LibraryPlaybackAssets {
            error: Some("Recording is not available for playback.".to_string()),
            recording_id: recording.id,
            segments: Vec::new(),
            session_id: String::new(),
        });
    }

    playback_server.create_session(&recording, &segments)
}

#[tauri::command(rename = "closeLibraryPlaybackSession")]
pub fn close_library_playback_session(
    input: CloseLibraryPlaybackSessionRequest,
    playback_server: State<'_, PlaybackServer>,
) -> Result<(), String> {
    playback_server.close_session(&input.session_id)
}

#[tauri::command(rename = "prepareLibraryRecordingThumbnail")]
pub async fn prepare_library_recording_thumbnail(
    input: LibraryRecordingThumbnailRequest,
    storage: State<'_, AppStorage>,
) -> Result<LibraryRecordingThumbnail, String> {
    let recording = storage
        .with_connection(|connection| storage::get_recording(connection, &input.recording_id))
        .map_err(|error| error.to_string())?;
    let segments = storage
        .with_connection(|connection| {
            storage::list_recording_segments(connection, &input.recording_id)
        })
        .map_err(|error| error.to_string())?;
    let detail = detail_from_rows(&recording, &segments, &[]);
    if detail.availability != LibraryRecordingAvailability::Available {
        return Ok(LibraryRecordingThumbnail {
            error: Some("Recording is not available for thumbnails.".to_string()),
            path: None,
            recording_id: recording.id,
        });
    }

    let recording_id = recording.id.clone();
    match tauri::async_runtime::spawn_blocking(move || {
        recording_thumbnail_for_recording(&recording, &segments)
    })
    .await
    .map_err(|error| error.to_string())?
    {
        Ok(path) => Ok(LibraryRecordingThumbnail {
            error: None,
            path,
            recording_id,
        }),
        Err(error) => Ok(LibraryRecordingThumbnail {
            error: Some(error),
            path: None,
            recording_id,
        }),
    }
}

fn detail_from_rows(
    recording: &RecordingRow,
    segments: &[RecordingSegmentRow],
    redactions: &[RedactionRow],
) -> LibraryRecordingDetail {
    let summary = summary_from_rows(recording, segments);
    let library_segments = library_segments(segments);
    let tracks = tracks_from_segments(segments);
    let library_redactions = library_redactions(redactions, &tracks);

    LibraryRecordingDetail {
        availability: summary.availability,
        duration_ms: summary.duration_ms,
        ended_at_ms: summary.ended_at_ms,
        id: summary.id,
        missing_reasons: summary.missing_reasons,
        playable_segment_count: summary.playable_segment_count,
        redactions: library_redactions,
        segments: library_segments,
        size_bytes: summary.size_bytes,
        started_at_ms: summary.started_at_ms,
        status: summary.status,
        storage_dir: summary.storage_dir,
        thumbnail_path: summary.thumbnail_path,
        title: summary.title,
        tracks,
    }
}

fn detail_for_recording(
    storage: &AppStorage,
    recording_id: &str,
) -> Result<LibraryRecordingDetail, String> {
    let recording = storage
        .with_connection(|connection| storage::get_recording(connection, recording_id))
        .map_err(|error| error.to_string())?;
    let segments = storage
        .with_connection(|connection| storage::list_recording_segments(connection, recording_id))
        .map_err(|error| error.to_string())?;
    let redactions = storage
        .with_connection(|connection| storage::list_redactions(connection, recording_id))
        .map_err(|error| error.to_string())?;

    Ok(detail_from_rows(&recording, &segments, &redactions))
}

fn recording_row_json(row: &RecordingRow) -> Value {
    json!({
        "activeSceneId": row.active_scene_id,
        "createdAtMs": row.created_at_ms,
        "defaultFps": row.default_fps,
        "endedAtMs": row.ended_at_ms,
        "id": row.id,
        "outputRoot": row.output_root,
        "qualityLabel": row.quality_label,
        "startedAtMs": row.started_at_ms,
        "status": row.status,
        "storageDir": row.storage_dir,
        "title": row.title,
    })
}

fn recording_segment_row_json(row: &RecordingSegmentRow) -> Value {
    json!({
        "desktopAudioStreamIndex": row.desktop_audio_stream_index,
        "durationMs": row.duration_ms,
        "endedAtMs": row.ended_at_ms,
        "errorCode": row.error_code,
        "ffmpegExitCode": row.ffmpeg_exit_code,
        "filePath": row.file_path,
        "id": row.id,
        "microphoneAudioStreamIndex": row.microphone_audio_stream_index,
        "probeJson": row.probe_json,
        "recordingId": row.recording_id,
        "segmentIndex": row.segment_index,
        "startedAtMs": row.started_at_ms,
        "state": row.state,
        "videoStreamIndex": row.video_stream_index,
    })
}

fn recording_track_row_json(row: &RecordingTrackRow) -> Value {
    json!({
        "codec": row.codec,
        "createdAtMs": row.created_at_ms,
        "id": row.id,
        "kind": row.kind,
        "label": row.label,
        "path": row.path,
        "recordingId": row.recording_id,
        "sourceId": row.source_id,
    })
}

fn redaction_row_json(row: &RedactionRow) -> Value {
    json!({
        "createdAtMs": row.created_at_ms,
        "endOffsetMs": row.end_offset_ms,
        "id": row.id,
        "reason": row.reason,
        "recordingId": row.recording_id,
        "startOffsetMs": row.start_offset_ms,
        "trackIdsJson": row.track_ids_json,
    })
}

fn processing_job_row_json(row: &ProcessingJobRow) -> Value {
    json!({
        "createdAtMs": row.created_at_ms,
        "error": row.error,
        "id": row.id,
        "kind": row.kind,
        "recordingId": row.recording_id,
        "status": row.status,
        "updatedAtMs": row.updated_at_ms,
    })
}

fn processing_chunk_row_json(row: &ProcessingChunkRow) -> Value {
    json!({
        "attemptCount": row.attempt_count,
        "createdAtMs": row.created_at_ms,
        "endOffsetMs": row.end_offset_ms,
        "errorCode": row.error_code,
        "id": row.id,
        "jobId": row.job_id,
        "progress": row.progress,
        "recordingId": row.recording_id,
        "segmentId": row.segment_id,
        "sourceKind": row.source_kind,
        "startOffsetMs": row.start_offset_ms,
        "status": row.status,
        "trackId": row.track_id,
        "updatedAtMs": row.updated_at_ms,
    })
}

fn transcript_segment_row_json(row: &TranscriptSegmentRow) -> Value {
    json!({
        "chunkId": row.chunk_id,
        "confidence": row.confidence,
        "createdAtMs": row.created_at_ms,
        "endOffsetMs": row.end_offset_ms,
        "id": row.id,
        "jobId": row.job_id,
        "model": row.model,
        "provider": row.provider,
        "recordingId": row.recording_id,
        "segmentId": row.segment_id,
        "sourceRole": row.source_role,
        "startOffsetMs": row.start_offset_ms,
        "text": row.text,
        "trackId": row.track_id,
    })
}

fn transcript_word_row_json(row: &TranscriptWordRow) -> Value {
    json!({
        "endOffsetMs": row.end_offset_ms,
        "id": row.id,
        "probability": row.probability,
        "startOffsetMs": row.start_offset_ms,
        "text": row.text,
        "transcriptSegmentId": row.transcript_segment_id,
    })
}

fn summary_from_rows(
    recording: &RecordingRow,
    segments: &[RecordingSegmentRow],
) -> LibraryRecordingSummary {
    let storage_dir_exists = Path::new(&recording.storage_dir).is_dir();
    let playable_segments = playable_segments(segments);
    let playable_segment_count = playable_segments.len();
    let mut missing_reasons = Vec::new();

    if storage::is_active_recording_status(&recording.status) {
        missing_reasons.push("Recording is still in progress.".to_string());
    } else if !storage_dir_exists {
        missing_reasons.push("Recording directory is missing.".to_string());
    }
    for segment in playable_segments.iter().filter(|segment| {
        !storage::is_active_recording_status(&recording.status)
            && !Path::new(&segment.file_path).is_file()
    }) {
        missing_reasons.push(format!(
            "Segment {} video file is missing.",
            segment.segment_index
        ));
    }

    let availability = if storage::is_active_recording_status(&recording.status) {
        LibraryRecordingAvailability::Locked
    } else if !missing_reasons.is_empty() {
        LibraryRecordingAvailability::MissingFiles
    } else if playable_segment_count == 0 {
        LibraryRecordingAvailability::Incomplete
    } else {
        LibraryRecordingAvailability::Available
    };

    LibraryRecordingSummary {
        availability,
        duration_ms: recording_duration_ms(recording, segments),
        ended_at_ms: recording.ended_at_ms,
        id: recording.id.clone(),
        missing_reasons,
        playable_segment_count,
        segment_count: segments.len(),
        size_bytes: recording_size_bytes(recording, segments),
        started_at_ms: recording.started_at_ms,
        status: recording.status.clone(),
        storage_dir: recording.storage_dir.clone(),
        thumbnail_path: playable_segments
            .iter()
            .find(|segment| Path::new(&segment.file_path).is_file())
            .map(|segment| segment.file_path.clone()),
        title: recording.title.clone(),
    }
}

fn library_redactions(
    redactions: &[RedactionRow],
    tracks: &[LibraryRecordingTrack],
) -> Vec<LibraryRecordingRedaction> {
    let all_track_ids = tracks
        .iter()
        .map(|track| track.id.clone())
        .collect::<Vec<_>>();

    redactions
        .iter()
        .map(|redaction| {
            let track_ids = parse_redaction_track_ids(&redaction.track_ids_json)
                .filter(|track_ids| !track_ids.is_empty())
                .unwrap_or_else(|| all_track_ids.clone());

            LibraryRecordingRedaction {
                created_at_ms: redaction.created_at_ms,
                end_offset_ms: redaction.end_offset_ms,
                id: redaction.id.clone(),
                reason: redaction.reason.clone(),
                recording_id: redaction.recording_id.clone(),
                start_offset_ms: redaction.start_offset_ms,
                track_ids,
            }
        })
        .collect()
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RedactionInterval {
    end_offset_ms: i64,
    start_offset_ms: i64,
}

fn build_normalized_redaction_inputs(
    recording_id: &str,
    ranges: &[CreateLibraryRedactionRangeInput],
    existing_redactions: &[RedactionRow],
    tracks: &[LibraryRecordingTrack],
    duration_ms: i64,
) -> Result<Vec<storage::CreateRedactionInput>, String> {
    if ranges.is_empty() {
        return Err("At least one redaction range is required.".to_string());
    }
    let valid_track_ids = tracks
        .iter()
        .map(|track| track.id.as_str())
        .collect::<BTreeSet<_>>();
    if valid_track_ids.is_empty() {
        return Err("Recording has no playable tracks to redact.".to_string());
    }
    let timestamp_ms = now_ms();
    let all_track_ids = tracks
        .iter()
        .map(|track| track.id.clone())
        .collect::<Vec<_>>();
    let mut intervals_by_track: BTreeMap<String, Vec<RedactionInterval>> = BTreeMap::new();

    for redaction in existing_redactions {
        if redaction.end_offset_ms <= redaction.start_offset_ms {
            continue;
        }
        let track_ids = parse_redaction_track_ids(&redaction.track_ids_json)
            .filter(|track_ids| !track_ids.is_empty())
            .unwrap_or_else(|| all_track_ids.clone());
        for track_id in track_ids
            .into_iter()
            .filter(|track_id| valid_track_ids.contains(track_id.as_str()))
        {
            intervals_by_track
                .entry(track_id)
                .or_default()
                .push(RedactionInterval {
                    end_offset_ms: redaction.end_offset_ms,
                    start_offset_ms: redaction.start_offset_ms,
                });
        }
    }

    for range in ranges {
        if range.end_offset_ms <= range.start_offset_ms {
            return Err("Redaction range must have positive duration.".to_string());
        }
        if range.start_offset_ms < 0 || range.end_offset_ms > duration_ms {
            return Err(format!(
                "Redaction range must be within recording duration {} ms.",
                duration_ms.max(0)
            ));
        }
        let track_ids = normalized_track_ids(&range.track_ids);
        if track_ids.is_empty() {
            return Err("Redaction requires at least one target track.".to_string());
        }
        if let Some(track_id) = track_ids
            .iter()
            .find(|track_id| !valid_track_ids.contains(track_id.as_str()))
        {
            return Err(format!("Redaction target track {track_id} does not exist."));
        }

        for track_id in track_ids {
            intervals_by_track
                .entry(track_id)
                .or_default()
                .push(RedactionInterval {
                    end_offset_ms: range.end_offset_ms,
                    start_offset_ms: range.start_offset_ms,
                });
        }
    }

    let mut track_ids_by_interval: BTreeMap<(i64, i64), Vec<String>> = BTreeMap::new();
    for (track_id, intervals) in intervals_by_track {
        for interval in merge_redaction_intervals(intervals) {
            track_ids_by_interval
                .entry((interval.start_offset_ms, interval.end_offset_ms))
                .or_default()
                .push(track_id.clone());
        }
    }

    track_ids_by_interval
        .into_iter()
        .map(|((start_offset_ms, end_offset_ms), track_ids)| {
            Ok(storage::CreateRedactionInput {
                created_at_ms: timestamp_ms,
                end_offset_ms,
                id: Uuid::now_v7().to_string(),
                reason: None,
                recording_id: recording_id.to_string(),
                start_offset_ms,
                track_ids_json: serde_json::to_string(&track_ids)
                    .map_err(|error| error.to_string())?,
            })
        })
        .collect()
}

fn build_unredacted_redaction_inputs(
    recording_id: &str,
    ranges: &[CreateLibraryRedactionRangeInput],
    existing_redactions: &[RedactionRow],
    tracks: &[LibraryRecordingTrack],
    duration_ms: i64,
) -> Result<Vec<storage::CreateRedactionInput>, String> {
    if ranges.is_empty() {
        return Err("At least one unredaction range is required.".to_string());
    }
    let valid_track_ids = tracks
        .iter()
        .map(|track| track.id.as_str())
        .collect::<BTreeSet<_>>();
    if valid_track_ids.is_empty() {
        return Err("Recording has no playable tracks to unredact.".to_string());
    }
    let all_track_ids = tracks
        .iter()
        .map(|track| track.id.clone())
        .collect::<Vec<_>>();
    let mut intervals_by_track: BTreeMap<String, Vec<RedactionInterval>> = BTreeMap::new();

    for redaction in existing_redactions {
        if redaction.end_offset_ms <= redaction.start_offset_ms {
            continue;
        }
        let track_ids = parse_redaction_track_ids(&redaction.track_ids_json)
            .filter(|track_ids| !track_ids.is_empty())
            .unwrap_or_else(|| all_track_ids.clone());
        for track_id in track_ids
            .into_iter()
            .filter(|track_id| valid_track_ids.contains(track_id.as_str()))
        {
            intervals_by_track
                .entry(track_id)
                .or_default()
                .push(RedactionInterval {
                    end_offset_ms: redaction.end_offset_ms,
                    start_offset_ms: redaction.start_offset_ms,
                });
        }
    }

    for range in ranges {
        if range.end_offset_ms <= range.start_offset_ms {
            return Err("Unredaction range must have positive duration.".to_string());
        }
        if range.start_offset_ms < 0 || range.end_offset_ms > duration_ms {
            return Err(format!(
                "Unredaction range must be within recording duration {} ms.",
                duration_ms.max(0)
            ));
        }
        let track_ids = normalized_track_ids(&range.track_ids);
        if track_ids.is_empty() {
            return Err("Unredaction requires at least one target track.".to_string());
        }
        if let Some(track_id) = track_ids
            .iter()
            .find(|track_id| !valid_track_ids.contains(track_id.as_str()))
        {
            return Err(format!(
                "Unredaction target track {track_id} does not exist."
            ));
        }

        let removal = RedactionInterval {
            end_offset_ms: range.end_offset_ms,
            start_offset_ms: range.start_offset_ms,
        };
        for track_id in track_ids {
            let remaining = subtract_redaction_interval(
                intervals_by_track.remove(&track_id).unwrap_or_default(),
                &removal,
            );
            if !remaining.is_empty() {
                intervals_by_track.insert(track_id, remaining);
            }
        }
    }

    redaction_inputs_from_intervals(recording_id, intervals_by_track)
}

fn merge_redaction_intervals(mut intervals: Vec<RedactionInterval>) -> Vec<RedactionInterval> {
    intervals.sort_by(|left, right| {
        left.start_offset_ms
            .cmp(&right.start_offset_ms)
            .then(left.end_offset_ms.cmp(&right.end_offset_ms))
    });
    let mut merged: Vec<RedactionInterval> = Vec::new();

    for interval in intervals {
        let Some(current) = merged.last_mut() else {
            merged.push(interval);
            continue;
        };
        if interval.start_offset_ms <= current.end_offset_ms {
            current.end_offset_ms = current.end_offset_ms.max(interval.end_offset_ms);
            continue;
        }

        merged.push(interval);
    }

    merged
}

fn subtract_redaction_interval(
    intervals: Vec<RedactionInterval>,
    removal: &RedactionInterval,
) -> Vec<RedactionInterval> {
    let mut remaining = Vec::new();

    for interval in intervals {
        if removal.end_offset_ms <= interval.start_offset_ms
            || removal.start_offset_ms >= interval.end_offset_ms
        {
            remaining.push(interval);
            continue;
        }

        if removal.start_offset_ms > interval.start_offset_ms {
            remaining.push(RedactionInterval {
                end_offset_ms: removal.start_offset_ms.min(interval.end_offset_ms),
                start_offset_ms: interval.start_offset_ms,
            });
        }
        if removal.end_offset_ms < interval.end_offset_ms {
            remaining.push(RedactionInterval {
                end_offset_ms: interval.end_offset_ms,
                start_offset_ms: removal.end_offset_ms.max(interval.start_offset_ms),
            });
        }
    }

    merge_redaction_intervals(remaining)
}

fn redaction_inputs_from_intervals(
    recording_id: &str,
    intervals_by_track: BTreeMap<String, Vec<RedactionInterval>>,
) -> Result<Vec<storage::CreateRedactionInput>, String> {
    let timestamp_ms = now_ms();
    let mut track_ids_by_interval: BTreeMap<(i64, i64), Vec<String>> = BTreeMap::new();
    for (track_id, intervals) in intervals_by_track {
        for interval in merge_redaction_intervals(intervals) {
            track_ids_by_interval
                .entry((interval.start_offset_ms, interval.end_offset_ms))
                .or_default()
                .push(track_id.clone());
        }
    }

    track_ids_by_interval
        .into_iter()
        .map(|((start_offset_ms, end_offset_ms), track_ids)| {
            Ok(storage::CreateRedactionInput {
                created_at_ms: timestamp_ms,
                end_offset_ms,
                id: Uuid::now_v7().to_string(),
                reason: None,
                recording_id: recording_id.to_string(),
                start_offset_ms,
                track_ids_json: serde_json::to_string(&track_ids)
                    .map_err(|error| error.to_string())?,
            })
        })
        .collect()
}

fn ensure_recording_allows_redaction(recording: &RecordingRow) -> Result<(), String> {
    if storage::is_active_recording_status(&recording.status) {
        return Err(
            "Recording is still controlled by the recorder and cannot be redacted yet.".to_string(),
        );
    }

    Ok(())
}

fn parse_redaction_track_ids(track_ids_json: &str) -> Option<Vec<String>> {
    serde_json::from_str::<Vec<String>>(track_ids_json)
        .ok()
        .map(|track_ids| normalized_track_ids(&track_ids))
}

fn normalized_track_ids(track_ids: &[String]) -> Vec<String> {
    track_ids
        .iter()
        .map(|track_id| track_id.trim())
        .filter(|track_id| !track_id.is_empty())
        .map(ToString::to_string)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn normalize_recording_title(title: Option<String>) -> Result<Option<String>, String> {
    let Some(title) = title else {
        return Ok(None);
    };
    let trimmed = title.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if trimmed.chars().count() > RECORDING_TITLE_MAX_CHARS {
        return Err(format!(
            "Recording title must be {RECORDING_TITLE_MAX_CHARS} characters or fewer."
        ));
    }

    Ok(Some(trimmed.to_string()))
}

fn delete_recording_row(
    storage: &AppStorage,
    recordings_root: &Path,
    recording: &RecordingRow,
) -> Result<(), String> {
    let storage_dir = PathBuf::from(&recording.storage_dir);

    if storage_dir.exists() {
        ensure_path_inside_recordings_root(recordings_root, &storage_dir)?;
        fs::remove_dir_all(&storage_dir).map_err(|error| {
            format!(
                "Unable to delete recording directory {}: {error}",
                path_to_string(&storage_dir)
            )
        })?;
    }

    storage
        .with_connection(|connection| storage::delete_recording(connection, &recording.id))
        .map_err(|error| error.to_string())
}

fn ensure_recording_is_deletable(recording: &RecordingRow) -> Result<(), String> {
    if storage::is_deletable_recording_status(&recording.status) {
        return Ok(());
    }

    Err(format!(
        "Recording {} is {} and cannot be deleted until capture has finished or recovered.",
        recording.id, recording.status
    ))
}

fn unique_recording_ids(recording_ids: Vec<String>) -> Vec<String> {
    let mut unique = Vec::new();
    for recording_id in recording_ids {
        if !unique.contains(&recording_id) {
            unique.push(recording_id);
        }
    }

    unique
}

fn recording_thumbnail_for_recording(
    recording: &RecordingRow,
    segments: &[RecordingSegmentRow],
) -> Result<Option<String>, String> {
    let Some(segment) = playable_segments(segments)
        .into_iter()
        .find(|segment| Path::new(&segment.file_path).is_file())
    else {
        return Ok(None);
    };
    let Some(video_stream_index) = segment.video_stream_index else {
        return Ok(None);
    };

    let ffmpeg_path = ffmpeg::binary_path();
    let segment_dir = Path::new(&recording.storage_dir)
        .join("playback")
        .join(&segment.id);
    fs::create_dir_all(&segment_dir).map_err(|error| {
        format!(
            "Unable to create playback cache directory {}: {error}",
            path_to_string(&segment_dir)
        )
    })?;

    let thumbnail_path = segment_dir.join("overview.jpg");
    ensure_timeline_thumbnail(
        &ffmpeg_path,
        segment,
        video_stream_index,
        0,
        &thumbnail_path,
    )?;

    Ok(Some(path_to_string(&thumbnail_path)))
}

fn ensure_timeline_thumbnail(
    ffmpeg_path: &str,
    segment: &RecordingSegmentRow,
    video_stream_index: i64,
    offset_ms: i64,
    output_path: &Path,
) -> Result<(), String> {
    if playback_asset_is_fresh(&segment.file_path, output_path) {
        return Ok(());
    }

    run_ffmpeg(
        ffmpeg_path,
        &[
            "-hide_banner",
            "-y",
            "-i",
            &segment.file_path,
            "-ss",
            &format!("{:.3}", offset_ms as f64 / 1000.0),
            "-map",
            &format!("0:{video_stream_index}"),
            "-frames:v",
            "1",
            "-an",
            "-vf",
            "scale=320:-1",
            "-q:v",
            "4",
            "-update",
            "1",
        ],
        output_path,
        "image2",
    )
}

fn playback_asset_is_fresh(source_path: &str, output_path: &Path) -> bool {
    let Some((source_len, source_modified_ms)) = file_fingerprint(source_path) else {
        return false;
    };
    let Some((output_len, output_modified_ms)) = file_fingerprint(&path_to_string(output_path))
    else {
        return false;
    };

    source_len > 0 && output_len > 0 && output_modified_ms >= source_modified_ms
}

fn run_ffmpeg(
    ffmpeg_path: &str,
    input_args: &[&str],
    output_path: &Path,
    output_format: &str,
) -> Result<(), String> {
    let temp_path = temp_output_path(output_path);
    let _ = fs::remove_file(&temp_path);
    let mut command = Command::new(ffmpeg_path);
    command
        .args(input_args)
        .args(["-f", output_format])
        .arg(&temp_path);
    let output = command.output().map_err(|error| {
        format!(
            "Unable to run FFmpeg at {ffmpeg_path}. Set DAYDREAM_FFMPEG_PATH or install ffmpeg: {error}"
        )
    })?;

    if !output.status.success() {
        let _ = fs::remove_file(&temp_path);
        return Err(format!(
            "FFmpeg failed while preparing playback assets: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    if !temp_path.is_file() || fs::metadata(&temp_path).map_or(0, |metadata| metadata.len()) == 0 {
        let _ = fs::remove_file(&temp_path);
        return Err(format!(
            "FFmpeg completed without creating playback asset {}: {}",
            path_to_string(output_path),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    fs::rename(&temp_path, output_path).map_err(|error| {
        format!(
            "Unable to move playback asset {} into place: {error}",
            path_to_string(output_path)
        )
    })
}

fn temp_output_path(output_path: &Path) -> PathBuf {
    let Some(extension) = output_path
        .extension()
        .and_then(|extension| extension.to_str())
    else {
        return output_path.with_extension("tmp");
    };

    output_path.with_extension(format!("tmp.{extension}"))
}

fn recording_size_bytes(recording: &RecordingRow, segments: &[RecordingSegmentRow]) -> u64 {
    let storage_dir = Path::new(&recording.storage_dir);
    if storage_dir.is_dir() {
        return directory_size_bytes(storage_dir).unwrap_or(0);
    }

    segments
        .iter()
        .filter_map(|segment| fs::metadata(&segment.file_path).ok())
        .map(|metadata| metadata.len())
        .sum()
}

fn directory_size_bytes(path: &Path) -> Result<u64, String> {
    let mut total = 0;
    for entry in fs::read_dir(path).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let metadata = entry.metadata().map_err(|error| error.to_string())?;
        if metadata.is_dir() {
            total += directory_size_bytes(&entry.path())?;
        } else {
            total += metadata.len();
        }
    }

    Ok(total)
}

fn library_segments(segments: &[RecordingSegmentRow]) -> Vec<LibraryRecordingSegment> {
    let mut playback_offset_ms = 0;
    let mut output = Vec::with_capacity(segments.len());

    for (index, segment) in segments.iter().enumerate() {
        let duration_ms = segment.duration_ms.unwrap_or_else(|| {
            segment
                .ended_at_ms
                .map(|ended_at_ms| ended_at_ms.saturating_sub(segment.started_at_ms))
                .unwrap_or(0)
        });
        let next_started_at_ms = segments.get(index + 1).map(|next| next.started_at_ms);
        let pause_after_ms = segment
            .ended_at_ms
            .zip(next_started_at_ms)
            .map(|(ended_at_ms, next_started_at_ms)| next_started_at_ms.saturating_sub(ended_at_ms))
            .unwrap_or(0);

        output.push(LibraryRecordingSegment {
            desktop_audio_stream_index: segment.desktop_audio_stream_index,
            duration_ms: segment.duration_ms,
            ended_at_ms: segment.ended_at_ms,
            file_exists: Path::new(&segment.file_path).is_file(),
            file_path: segment.file_path.clone(),
            id: segment.id.clone(),
            microphone_audio_stream_index: segment.microphone_audio_stream_index,
            pause_after_ms,
            playback_offset_ms,
            segment_index: segment.segment_index,
            started_at_ms: segment.started_at_ms,
            state: segment.state.clone(),
            video_stream_index: segment.video_stream_index,
        });
        if segment.state == "finalized" && Path::new(&segment.file_path).is_file() {
            playback_offset_ms += duration_ms.max(0) + pause_after_ms.max(0);
        }
    }

    output
}

fn tracks_from_segments(segments: &[RecordingSegmentRow]) -> Vec<LibraryRecordingTrack> {
    let mut tracks: BTreeMap<(LibraryTrackKind, i64), LibraryRecordingTrack> = BTreeMap::new();

    for segment in playable_segments(segments) {
        let probe = segment_probe(segment);
        if let Some(index) = segment.video_stream_index {
            tracks
                .entry((LibraryTrackKind::Video, index))
                .or_insert_with(|| LibraryRecordingTrack {
                    codec: probe
                        .as_ref()
                        .and_then(|probe| probe.video.as_ref())
                        .and_then(|video| (video.index == index).then(|| video.codec_name.clone()))
                        .flatten(),
                    description: "Composited screen recording".to_string(),
                    id: format!("video-{index}"),
                    kind: LibraryTrackKind::Video,
                    label: "Video".to_string(),
                    stream_index: Some(index),
                });
        }
        if let Some(index) = segment.desktop_audio_stream_index {
            tracks
                .entry((LibraryTrackKind::DesktopAudio, index))
                .or_insert_with(|| {
                    audio_track(
                        "desktop-audio",
                        LibraryTrackKind::DesktopAudio,
                        "Desktop audio",
                        index,
                        probe.as_ref(),
                    )
                });
        }
        if let Some(index) = segment.microphone_audio_stream_index {
            tracks
                .entry((LibraryTrackKind::Microphone, index))
                .or_insert_with(|| {
                    audio_track(
                        "microphone",
                        LibraryTrackKind::Microphone,
                        "Microphone",
                        index,
                        probe.as_ref(),
                    )
                });
        }
    }

    tracks.into_values().collect()
}

fn audio_track(
    id_prefix: &str,
    kind: LibraryTrackKind,
    label: &str,
    index: i64,
    probe: Option<&SegmentProbe>,
) -> LibraryRecordingTrack {
    LibraryRecordingTrack {
        codec: probe
            .and_then(|probe| probe.audio.iter().find(|audio| audio.index == index))
            .and_then(|audio| audio.codec_name.clone()),
        description: format!("{label} stream {index}"),
        id: format!("{id_prefix}-{index}"),
        kind,
        label: label.to_string(),
        stream_index: Some(index),
    }
}

fn playable_segments(segments: &[RecordingSegmentRow]) -> Vec<&RecordingSegmentRow> {
    segments
        .iter()
        .filter(|segment| segment.state == "finalized")
        .collect()
}

fn recording_duration_ms(
    recording: &RecordingRow,
    segments: &[RecordingSegmentRow],
) -> Option<i64> {
    if let (Some(started_at_ms), Some(ended_at_ms)) =
        (recording.started_at_ms, recording.ended_at_ms)
    {
        return Some(ended_at_ms.saturating_sub(started_at_ms));
    }

    let total = playable_segments(segments)
        .iter()
        .filter_map(|segment| segment.duration_ms)
        .sum::<i64>();

    (total > 0).then_some(total)
}

fn timeline_duration_ms(segments: &[RecordingSegmentRow]) -> i64 {
    library_segments(segments)
        .into_iter()
        .map(|segment| segment.playback_offset_ms + segment.duration_ms.unwrap_or_default())
        .max()
        .unwrap_or_default()
        .max(0)
}

fn segment_probe(segment: &RecordingSegmentRow) -> Option<SegmentProbe> {
    segment
        .probe_json
        .as_ref()
        .and_then(|probe_json| serde_json::from_str(probe_json).ok())
}

fn file_fingerprint(path: &str) -> Option<(u64, i64)> {
    let metadata = fs::metadata(path).ok()?;
    let modified_ms = metadata
        .modified()
        .ok()
        .and_then(system_time_ms)
        .unwrap_or_default();

    Some((metadata.len(), modified_ms))
}

fn system_time_ms(time: SystemTime) -> Option<i64> {
    time.duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_millis() as i64)
}

fn now_ms() -> i64 {
    system_time_ms(SystemTime::now()).unwrap_or_default()
}

fn ensure_path_inside_recordings_root(recordings_root: &Path, path: &Path) -> Result<(), String> {
    let canonical_root = recordings_root.canonicalize().map_err(|error| {
        format!(
            "Unable to inspect recordings root {}: {error}",
            path_to_string(recordings_root)
        )
    })?;
    let canonical_path = path.canonicalize().map_err(|error| {
        format!(
            "Unable to inspect recording directory {}: {error}",
            path_to_string(path)
        )
    })?;
    if !canonical_path.starts_with(canonical_root) {
        return Err(
            "Refusing to delete a recording directory outside Daydream storage.".to_string(),
        );
    }

    Ok(())
}

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::waveforms::{
        parse_volumedetect_max_amplitude, waveform_bins, waveform_peak_rms_bins,
        waveform_tile_bin_ms, waveform_tile_ranges_for_segment, waveform_tile_sample_count,
        waveforms_for_recording,
    };
    use super::*;

    #[test]
    fn summary_marks_missing_segment_files() {
        let temp_dir = test_dir("missing-segment-files");
        fs::create_dir_all(&temp_dir).expect("create temp recording dir");
        let recording = recording_row(&temp_dir);
        let segments = vec![segment_row(temp_dir.join("segments/0001.mkv"), "finalized")];

        let summary = summary_from_rows(&recording, &segments);

        let _ = fs::remove_dir_all(temp_dir);
        assert_eq!(
            summary.availability,
            LibraryRecordingAvailability::MissingFiles
        );
        assert_eq!(summary.playable_segment_count, 1);
        assert!(summary
            .missing_reasons
            .iter()
            .any(|reason| reason.contains("video file is missing")));
    }

    #[test]
    fn summary_marks_recordings_without_finalized_segments_incomplete() {
        let temp_dir = test_dir("incomplete-recording");
        fs::create_dir_all(&temp_dir).expect("create temp recording dir");
        let recording = recording_row(&temp_dir);
        let segments = vec![segment_row(temp_dir.join("segments/0001.mkv"), "recording")];

        let summary = summary_from_rows(&recording, &segments);

        let _ = fs::remove_dir_all(temp_dir);
        assert_eq!(
            summary.availability,
            LibraryRecordingAvailability::Incomplete
        );
        assert_eq!(summary.playable_segment_count, 0);
    }

    #[test]
    fn summary_marks_active_recordings_locked() {
        let temp_dir = test_dir("locked-recording");
        fs::create_dir_all(&temp_dir).expect("create temp recording dir");
        let mut recording = recording_row(&temp_dir);
        recording.status = "recording".to_string();
        let segments = vec![segment_row(temp_dir.join("segments/0001.mkv"), "recording")];

        let summary = summary_from_rows(&recording, &segments);

        let _ = fs::remove_dir_all(temp_dir);
        assert_eq!(summary.availability, LibraryRecordingAvailability::Locked);
        assert!(summary
            .missing_reasons
            .iter()
            .any(|reason| reason.contains("in progress")));
    }

    #[test]
    fn summary_and_detail_include_recording_title() {
        let temp_dir = test_dir("recording-title");
        fs::create_dir_all(&temp_dir).expect("create temp recording dir");
        let mut recording = recording_row(&temp_dir);
        recording.title = Some("Workday review".to_string());
        let segment_path = temp_dir.join("segments/0001.mkv");
        fs::create_dir_all(segment_path.parent().expect("segment parent"))
            .expect("create segment dir");
        fs::write(&segment_path, [1_u8]).expect("write segment");
        let segments = vec![segment_row(segment_path, "finalized")];

        let summary = summary_from_rows(&recording, &segments);
        let detail = detail_from_rows(&recording, &segments, &[]);

        let _ = fs::remove_dir_all(temp_dir);
        assert_eq!(summary.title.as_deref(), Some("Workday review"));
        assert_eq!(detail.title.as_deref(), Some("Workday review"));
    }

    #[test]
    fn normalize_recording_title_trims_clears_and_rejects_long_values() {
        assert_eq!(
            normalize_recording_title(Some("  Workday review  ".to_string())).expect("trim"),
            Some("Workday review".to_string()),
        );
        assert_eq!(
            normalize_recording_title(Some("   ".to_string())).expect("clear"),
            None,
        );
        assert!(
            normalize_recording_title(Some("x".repeat(RECORDING_TITLE_MAX_CHARS + 1)))
                .expect_err("long title should fail")
                .contains("characters or fewer")
        );
    }

    #[test]
    fn new_redactions_merge_with_existing_redactions_per_track() {
        let tracks = vec![
            library_track("video-0", LibraryTrackKind::Video),
            library_track("desktop-audio-1", LibraryTrackKind::DesktopAudio),
            library_track("microphone-2", LibraryTrackKind::Microphone),
        ];
        let existing_redactions = vec![
            redaction_row("existing-video", 24_000, 26_000, &["video-0"]),
            redaction_row(
                "existing-desktop-audio",
                23_000,
                24_000,
                &["desktop-audio-1"],
            ),
        ];

        let normalized = build_normalized_redaction_inputs(
            "recording-1",
            &[CreateLibraryRedactionRangeInput {
                end_offset_ms: 25_000,
                start_offset_ms: 23_500,
                track_ids: vec![
                    "desktop-audio-1".to_string(),
                    "microphone-2".to_string(),
                    "video-0".to_string(),
                ],
            }],
            &existing_redactions,
            &tracks,
            30_000,
        )
        .expect("normalize redactions");

        let normalized_ranges = normalized
            .iter()
            .map(|redaction| {
                (
                    redaction.start_offset_ms,
                    redaction.end_offset_ms,
                    parse_redaction_track_ids(&redaction.track_ids_json).expect("track ids"),
                )
            })
            .collect::<Vec<_>>();

        assert_eq!(
            normalized_ranges,
            vec![
                (23_000, 25_000, vec!["desktop-audio-1".to_string()],),
                (23_500, 25_000, vec!["microphone-2".to_string()]),
                (23_500, 26_000, vec!["video-0".to_string()]),
            ]
        );
    }

    #[test]
    fn unredaction_range_splits_existing_redactions_per_track() {
        let tracks = vec![
            library_track("video-0", LibraryTrackKind::Video),
            library_track("desktop-audio-1", LibraryTrackKind::DesktopAudio),
        ];
        let existing_redactions = vec![redaction_row(
            "existing",
            20_000,
            26_000,
            &["desktop-audio-1", "video-0"],
        )];

        let normalized = build_unredacted_redaction_inputs(
            "recording-1",
            &[CreateLibraryRedactionRangeInput {
                end_offset_ms: 24_000,
                start_offset_ms: 22_000,
                track_ids: vec!["desktop-audio-1".to_string(), "video-0".to_string()],
            }],
            &existing_redactions,
            &tracks,
            30_000,
        )
        .expect("unredact range");

        let normalized_ranges = normalized
            .iter()
            .map(|redaction| {
                (
                    redaction.start_offset_ms,
                    redaction.end_offset_ms,
                    parse_redaction_track_ids(&redaction.track_ids_json).expect("track ids"),
                )
            })
            .collect::<Vec<_>>();

        assert_eq!(
            normalized_ranges,
            vec![
                (
                    20_000,
                    22_000,
                    vec!["desktop-audio-1".to_string(), "video-0".to_string()],
                ),
                (
                    24_000,
                    26_000,
                    vec!["desktop-audio-1".to_string(), "video-0".to_string()],
                ),
            ]
        );
    }

    #[test]
    fn debug_tools_are_disabled_by_default() {
        let storage = storage::test_storage(PathBuf::new());

        let error = ensure_debug_tools_enabled(&storage).expect_err("debug should be disabled");

        assert_eq!(error, "Debug tools are disabled.");
    }

    #[test]
    fn debug_tools_are_enabled_by_persisted_setting() {
        let storage = storage::test_storage(PathBuf::new());
        storage
            .with_connection(|connection| {
                storage::save_app_setting(
                    connection,
                    &storage::SaveAppSettingInput {
                        is_sensitive: false,
                        key: "debug.enabled".to_string(),
                        timestamp_ms: 1,
                        value_json: "true".to_string(),
                    },
                )
            })
            .expect("save debug setting");

        ensure_debug_tools_enabled(&storage).expect("debug should be enabled");
    }

    #[test]
    fn delete_guard_rejects_active_recordings() {
        let temp_dir = test_dir("delete-guard-active-recording");
        let mut recording = recording_row(&temp_dir);
        recording.status = "paused".to_string();

        let error = ensure_recording_is_deletable(&recording)
            .expect_err("active recordings cannot be deleted");

        assert!(error.contains("cannot be deleted"));
    }

    #[test]
    fn summary_reports_recursive_recording_directory_size() {
        let temp_dir = test_dir("recording-size");
        fs::create_dir_all(temp_dir.join("segments")).expect("create segments dir");
        fs::write(temp_dir.join("segments/0001.mkv"), [1_u8; 10]).expect("write segment");
        fs::write(temp_dir.join("metadata.json"), [1_u8; 5]).expect("write metadata");
        let recording = recording_row(&temp_dir);
        let segments = vec![segment_row(temp_dir.join("segments/0001.mkv"), "finalized")];

        let summary = summary_from_rows(&recording, &segments);

        let _ = fs::remove_dir_all(temp_dir);
        assert_eq!(summary.size_bytes, 15);
    }

    #[test]
    fn library_segments_preserve_pause_gaps_in_timeline() {
        let temp_dir = test_dir("pause-gaps");
        fs::create_dir_all(temp_dir.join("segments")).expect("create segments dir");
        let first_path = temp_dir.join("segments/0001.mkv");
        let second_path = temp_dir.join("segments/0002.mkv");
        fs::write(&first_path, "one").expect("write first segment");
        fs::write(&second_path, "two").expect("write second segment");
        let mut first = segment_row(first_path, "finalized");
        first.duration_ms = Some(2_000);
        first.ended_at_ms = Some(3_000);
        let mut second = segment_row(second_path, "finalized");
        second.id = "segment-2".to_string();
        second.segment_index = 2;
        second.started_at_ms = 6_000;
        second.ended_at_ms = Some(8_000);
        second.duration_ms = Some(2_000);

        let segments = library_segments(&[first, second]);

        let _ = fs::remove_dir_all(temp_dir);
        assert_eq!(segments[0].pause_after_ms, 3_000);
        assert_eq!(segments[1].playback_offset_ms, 5_000);
    }

    #[test]
    fn waveforms_do_not_write_cache_files() {
        let temp_dir = test_dir("waveform-no-cache");
        fs::create_dir_all(temp_dir.join("segments")).expect("create segments dir");
        let segment_path = temp_dir.join("segments/0001.mkv");
        fs::write(&segment_path, "media").expect("write segment");
        let recording = recording_row(&temp_dir);
        let segment = segment_row(segment_path, "finalized");

        let _waveforms = waveforms_for_recording(&recording, &[segment]);

        assert!(!temp_dir.join("waveforms/library-waveforms.json").exists());
        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn waveform_bins_return_peak_buckets() {
        let bins = waveform_bins(&[0.1, 0.8, 0.2, 0.4], 2);

        assert_eq!(bins, vec![0.8, 0.4]);
    }

    #[test]
    fn waveform_peak_rms_bins_return_native_amplitude_buckets() {
        let (peak_samples, rms_samples) = waveform_peak_rms_bins(&[0.3, -0.4, 0.6, -0.8], 2);

        assert_eq!(peak_samples, vec![0.4, 0.8]);
        assert!((rms_samples[0] - 0.353_553_390_593_273_8).abs() < f64::EPSILON);
        assert!((rms_samples[1] - std::f64::consts::FRAC_1_SQRT_2).abs() < f64::EPSILON);
    }

    #[test]
    fn waveform_tile_sample_count_keeps_short_non_empty_tiles_visible() {
        assert_eq!(waveform_tile_sample_count(10, 250, false), 2);
        assert_eq!(waveform_tile_sample_count(10, 250, true), 1);
        assert_eq!(waveform_tile_sample_count(1_000, 250, false), 4);
    }

    #[test]
    fn waveform_tile_bin_ms_tracks_visible_ms_per_pixel() {
        assert_eq!(
            waveform_tile_bin_ms(&LibraryWaveformTileRequest {
                end_timeline_ms: 10_000,
                recording_id: "recording-1".to_string(),
                start_timeline_ms: 0,
                viewport_width_px: 1_000.0,
            }),
            10
        );
        assert_eq!(
            waveform_tile_bin_ms(&LibraryWaveformTileRequest {
                end_timeline_ms: 3_000,
                recording_id: "recording-1".to_string(),
                start_timeline_ms: 0,
                viewport_width_px: 1_000.0,
            }),
            5
        );
        assert_eq!(
            waveform_tile_bin_ms(&LibraryWaveformTileRequest {
                end_timeline_ms: 260_000,
                recording_id: "recording-1".to_string(),
                start_timeline_ms: 0,
                viewport_width_px: 1_000.0,
            }),
            250
        );
    }

    #[test]
    fn waveform_tile_ranges_follow_segment_timeline_offsets() {
        let segment = LibraryRecordingSegment {
            desktop_audio_stream_index: Some(1),
            duration_ms: Some(10_000),
            ended_at_ms: Some(11_000),
            file_exists: true,
            file_path: "/tmp/daydream/segment.mkv".to_string(),
            id: "segment-1".to_string(),
            microphone_audio_stream_index: Some(2),
            pause_after_ms: 0,
            playback_offset_ms: 5_000,
            segment_index: 1,
            started_at_ms: 1_000,
            state: "finalized".to_string(),
            video_stream_index: Some(0),
        };

        let ranges = waveform_tile_ranges_for_segment(&segment, 7_500, 7_900);
        let outside_ranges = waveform_tile_ranges_for_segment(&segment, 1_000, 2_000);

        assert_eq!(ranges, vec![(0, 10_000)]);
        assert!(outside_ranges.is_empty());
    }

    #[test]
    fn parse_volumedetect_max_amplitude_reads_db_and_silence() {
        assert_eq!(
            parse_volumedetect_max_amplitude("[Parsed_volumedetect_0] max_volume: -6.0 dB\n"),
            Some(0.501_187_233_627_272_2)
        );
        assert_eq!(
            parse_volumedetect_max_amplitude("[Parsed_volumedetect_0] max_volume: -inf dB\n"),
            Some(0.0)
        );
    }

    #[test]
    fn playback_asset_cache_requires_existing_non_empty_output() {
        let temp_dir = test_dir("playback-cache");
        fs::create_dir_all(&temp_dir).expect("create temp dir");
        let source_path = temp_dir.join("segment.mkv");
        let output_path = temp_dir.join("video.mp4");
        fs::write(&source_path, "source").expect("write source");

        assert!(!playback_asset_is_fresh(
            &path_to_string(&source_path),
            &output_path
        ));

        fs::write(&output_path, "output").expect("write output");

        assert!(playback_asset_is_fresh(
            &path_to_string(&source_path),
            &output_path
        ));
        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn recording_thumbnail_returns_none_without_playable_segments() {
        let temp_dir = test_dir("recording-thumbnail-empty");
        fs::create_dir_all(&temp_dir).expect("create temp dir");
        let recording = recording_row(&temp_dir);
        let segment = segment_row(temp_dir.join("segments/0001.mkv"), "recording");

        let thumbnail = recording_thumbnail_for_recording(&recording, &[segment])
            .expect("thumbnail lookup should not fail");

        let _ = fs::remove_dir_all(temp_dir);
        assert_eq!(thumbnail, None);
    }

    #[test]
    fn temp_output_path_preserves_container_extension() {
        assert_eq!(
            temp_output_path(Path::new("/tmp/daydream/video.mp4")),
            PathBuf::from("/tmp/daydream/video.tmp.mp4")
        );
        assert_eq!(
            temp_output_path(Path::new("/tmp/daydream/audio.m4a")),
            PathBuf::from("/tmp/daydream/audio.tmp.m4a")
        );
        assert_eq!(
            temp_output_path(Path::new("/tmp/daydream/thumb.jpg")),
            PathBuf::from("/tmp/daydream/thumb.tmp.jpg")
        );
    }

    #[cfg(unix)]
    #[test]
    fn ffmpeg_success_without_temp_output_is_rejected() {
        use std::os::unix::fs::PermissionsExt;

        let temp_dir = test_dir("missing-ffmpeg-output");
        fs::create_dir_all(&temp_dir).expect("create temp dir");
        let fake_ffmpeg_path = temp_dir.join("fake-ffmpeg");
        let output_path = temp_dir.join("video.mp4");
        fs::write(&fake_ffmpeg_path, "#!/bin/sh\nexit 0\n").expect("write fake ffmpeg");
        fs::set_permissions(&fake_ffmpeg_path, fs::Permissions::from_mode(0o755))
            .expect("make fake ffmpeg executable");

        let error = run_ffmpeg(&path_to_string(&fake_ffmpeg_path), &[], &output_path, "mp4")
            .expect_err("missing temp output should fail");

        let _ = fs::remove_dir_all(temp_dir);
        assert!(error.contains("completed without creating playback asset"));
    }

    fn recording_row(storage_dir: &Path) -> RecordingRow {
        RecordingRow {
            active_scene_id: None,
            created_at_ms: 1_000,
            default_fps: 5.0,
            ended_at_ms: Some(5_000),
            id: "recording-1".to_string(),
            output_root: None,
            quality_label: "balanced-ocr".to_string(),
            started_at_ms: Some(1_000),
            status: "completed".to_string(),
            storage_dir: path_to_string(storage_dir),
            title: None,
        }
    }

    fn segment_row(file_path: PathBuf, state: &str) -> RecordingSegmentRow {
        RecordingSegmentRow {
            desktop_audio_stream_index: Some(1),
            duration_ms: Some(4_000),
            ended_at_ms: Some(5_000),
            error_code: None,
            ffmpeg_exit_code: Some(0),
            file_path: path_to_string(&file_path),
            id: "segment-1".to_string(),
            microphone_audio_stream_index: Some(2),
            probe_json: Some(
                serde_json::json!({
                    "audio": [
                        {"codecName": "opus", "index": 1},
                        {"codecName": "opus", "index": 2}
                    ],
                    "video": {"codecName": "h264", "index": 0}
                })
                .to_string(),
            ),
            recording_id: "recording-1".to_string(),
            segment_index: 1,
            started_at_ms: 1_000,
            state: state.to_string(),
            video_stream_index: Some(0),
        }
    }

    fn library_track(id: &str, kind: LibraryTrackKind) -> LibraryRecordingTrack {
        LibraryRecordingTrack {
            codec: None,
            description: id.to_string(),
            id: id.to_string(),
            kind,
            label: id.to_string(),
            stream_index: None,
        }
    }

    fn redaction_row(
        id: &str,
        start_offset_ms: i64,
        end_offset_ms: i64,
        track_ids: &[&str],
    ) -> RedactionRow {
        RedactionRow {
            created_at_ms: 1,
            end_offset_ms,
            id: id.to_string(),
            reason: None,
            recording_id: "recording-1".to_string(),
            start_offset_ms,
            track_ids_json: serde_json::to_string(track_ids).expect("track ids json"),
        }
    }

    fn test_dir(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("daydream-library-{name}-{}", std::process::id()))
    }
}
