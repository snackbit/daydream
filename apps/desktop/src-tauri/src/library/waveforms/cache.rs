use super::super::system_time_ms;
use crate::storage::{RecordingRow, RecordingSegmentRow};
use serde::{Deserialize, Serialize};
use std::{
    collections::hash_map::DefaultHasher,
    fs,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Serialize, Deserialize, Hash, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(super) struct WaveformFileFingerprint {
    pub(super) modified_ms: i64,
    pub(super) path: String,
    pub(super) size_bytes: u64,
}

pub(super) fn waveform_tile_cache_path(
    recording: &RecordingRow,
    segment: &RecordingSegmentRow,
    stream_index: i64,
    bin_ms: i64,
    tile_start_ms: i64,
    fingerprint: &WaveformFileFingerprint,
) -> PathBuf {
    Path::new(&recording.storage_dir)
        .join("waveforms")
        .join("v4")
        .join(format!("segment-{}", safe_cache_component(&segment.id)))
        .join(format!("stream-{stream_index}"))
        .join(format!("bin-{bin_ms}"))
        .join(format!(
            "tile-{tile_start_ms}-{}.json",
            hash_cache_key(fingerprint)
        ))
}
pub(super) fn waveform_file_fingerprint(path: &str) -> Result<WaveformFileFingerprint, String> {
    let metadata = fs::metadata(path).map_err(|_| "waveform_source_unavailable")?;
    let modified_ms = metadata
        .modified()
        .ok()
        .and_then(system_time_ms)
        .unwrap_or_default();
    Ok(WaveformFileFingerprint {
        modified_ms,
        path: path.to_owned(),
        size_bytes: metadata.len(),
    })
}
fn safe_cache_component(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
                character
            } else {
                '_'
            }
        })
        .collect()
}
fn hash_cache_key<T: Hash>(value: T) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}
