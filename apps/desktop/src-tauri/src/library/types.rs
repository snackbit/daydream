use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::ai;

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
    pub(crate) availability: LibraryRecordingAvailability,
    pub(crate) duration_ms: Option<i64>,
    pub(crate) ended_at_ms: Option<i64>,
    pub(crate) id: String,
    pub(crate) missing_reasons: Vec<String>,
    pub(crate) playable_segment_count: usize,
    pub(crate) segment_count: usize,
    pub(crate) size_bytes: u64,
    pub(crate) started_at_ms: Option<i64>,
    pub(crate) status: String,
    pub(crate) storage_dir: String,
    pub(crate) thumbnail_path: Option<String>,
    pub(crate) title: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryRecordingDetail {
    pub(crate) availability: LibraryRecordingAvailability,
    pub(crate) duration_ms: Option<i64>,
    pub(crate) ended_at_ms: Option<i64>,
    pub(crate) id: String,
    pub(crate) missing_reasons: Vec<String>,
    pub(crate) playable_segment_count: usize,
    pub(crate) redactions: Vec<LibraryRecordingRedaction>,
    pub(crate) segments: Vec<LibraryRecordingSegment>,
    pub(crate) size_bytes: u64,
    pub(crate) started_at_ms: Option<i64>,
    pub(crate) status: String,
    pub(crate) storage_dir: String,
    pub(crate) thumbnail_path: Option<String>,
    pub(crate) title: Option<String>,
    pub(crate) tracks: Vec<LibraryRecordingTrack>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryRecordingSegment {
    pub(crate) desktop_audio_stream_index: Option<i64>,
    pub(crate) duration_ms: Option<i64>,
    pub(crate) ended_at_ms: Option<i64>,
    pub(crate) file_exists: bool,
    pub(crate) file_path: String,
    pub(crate) id: String,
    pub(crate) microphone_audio_stream_index: Option<i64>,
    pub(crate) pause_after_ms: i64,
    pub(crate) playback_offset_ms: i64,
    pub(crate) segment_index: i64,
    pub(crate) started_at_ms: i64,
    pub(crate) state: String,
    pub(crate) video_stream_index: Option<i64>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryRecordingTrack {
    pub(crate) codec: Option<String>,
    pub(crate) description: String,
    pub(crate) id: String,
    pub(crate) kind: LibraryTrackKind,
    pub(crate) label: String,
    pub(crate) stream_index: Option<i64>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryRecordingRedaction {
    pub(crate) created_at_ms: i64,
    pub(crate) end_offset_ms: i64,
    pub(crate) id: String,
    pub(crate) reason: Option<String>,
    pub(crate) recording_id: String,
    pub(crate) start_offset_ms: i64,
    pub(crate) track_ids: Vec<String>,
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
    pub(crate) recording_id: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteLibraryRecordingsInput {
    pub(crate) recording_ids: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenameLibraryRecordingInput {
    pub(crate) recording_id: String,
    pub(crate) title: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateLibraryRedactionRangeInput {
    pub(crate) end_offset_ms: i64,
    pub(crate) start_offset_ms: i64,
    pub(crate) track_ids: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateLibraryRedactionsInput {
    pub(crate) recording_id: String,
    pub(crate) ranges: Vec<CreateLibraryRedactionRangeInput>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteLibraryRedactionInput {
    pub(crate) recording_id: String,
    pub(crate) redaction_id: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnredactLibraryRedactionsInput {
    pub(crate) recording_id: String,
    pub(crate) ranges: Vec<CreateLibraryRedactionRangeInput>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryWaveformRequest {
    pub(crate) recording_id: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryWaveformTileRequest {
    pub(crate) recording_id: String,
    pub(crate) start_timeline_ms: i64,
    pub(crate) end_timeline_ms: i64,
    pub(crate) viewport_width_px: f64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryPlaybackAssetRequest {
    pub(crate) recording_id: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloseLibraryPlaybackSessionRequest {
    pub(crate) session_id: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryRecordingThumbnail {
    pub(crate) error: Option<String>,
    pub(crate) path: Option<String>,
    pub(crate) recording_id: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryRecordingThumbnailRequest {
    pub(crate) recording_id: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryRecordingDebugInput {
    pub(crate) recording_id: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryRecordingDebugData {
    pub(crate) detail: LibraryRecordingDetail,
    pub(crate) processing_state: ai::RecordingAiProcessingState,
    pub(crate) raw: Value,
    pub(crate) recording_id: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryPlaybackAssets {
    pub(crate) error: Option<String>,
    pub(crate) recording_id: String,
    pub(crate) segments: Vec<LibraryPlaybackSegment>,
    pub(crate) session_id: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryPlaybackSegment {
    pub(crate) audio: Vec<LibraryPlaybackAudioAsset>,
    pub(crate) segment_id: String,
    pub(crate) thumbnails: Vec<LibraryPlaybackThumbnail>,
    pub(crate) video_url: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryPlaybackAudioAsset {
    pub(crate) kind: LibraryTrackKind,
    pub(crate) stream_index: i64,
    pub(crate) track_id: String,
    pub(crate) url: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryPlaybackThumbnail {
    pub(crate) offset_ms: i64,
    pub(crate) url: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryWaveforms {
    pub(crate) error: Option<String>,
    pub(crate) recording_id: String,
    pub(crate) tracks: Vec<LibraryWaveformTrack>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryWaveformTrack {
    pub(crate) kind: LibraryTrackKind,
    pub(crate) segments: Vec<LibraryWaveformSegment>,
    pub(crate) track_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryWaveformSegment {
    pub(crate) generated_at_ms: i64,
    pub(crate) max_amplitude: f64,
    pub(crate) samples: Vec<f64>,
    pub(crate) segment_id: String,
    pub(crate) stream_index: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LibraryWaveformTiles {
    pub(crate) bin_ms: i64,
    pub(crate) error: Option<String>,
    pub(crate) recording_id: String,
    pub(crate) tracks: Vec<LibraryWaveformTileTrack>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LibraryWaveformTileTrack {
    pub(crate) kind: LibraryTrackKind,
    pub(crate) scale_amplitude: f64,
    pub(crate) tiles: Vec<LibraryWaveformTile>,
    pub(crate) track_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LibraryWaveformTile {
    pub(crate) bin_ms: i64,
    pub(crate) duration_ms: i64,
    pub(crate) generated_at_ms: i64,
    pub(crate) peak_samples: Vec<f64>,
    pub(crate) rms_samples: Vec<f64>,
    pub(crate) segment_id: String,
    pub(crate) stream_index: i64,
    pub(crate) timeline_start_ms: i64,
}
