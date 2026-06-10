use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use tauri::{Manager, State};

use crate::{
    ffmpeg,
    playback_server::PlaybackServer,
    storage::{self, AppStorage, RecordingRow, RecordingSegmentRow},
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

    Ok(detail_from_rows(&recording, &segments))
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

    Ok(detail_from_rows(&recording, &segments))
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
    let detail = detail_from_rows(&recording, &segments);
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
    let detail = detail_from_rows(&recording, &segments);
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
) -> LibraryRecordingDetail {
    let summary = summary_from_rows(recording, segments);
    let library_segments = library_segments(segments);

    LibraryRecordingDetail {
        availability: summary.availability,
        duration_ms: summary.duration_ms,
        ended_at_ms: summary.ended_at_ms,
        id: summary.id,
        missing_reasons: summary.missing_reasons,
        playable_segment_count: summary.playable_segment_count,
        segments: library_segments,
        size_bytes: summary.size_bytes,
        started_at_ms: summary.started_at_ms,
        status: summary.status,
        storage_dir: summary.storage_dir,
        thumbnail_path: summary.thumbnail_path,
        title: summary.title,
        tracks: tracks_from_segments(segments),
    }
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
        let detail = detail_from_rows(&recording, &segments);

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

    fn test_dir(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("daydream-library-{name}-{}", std::process::id()))
    }
}
