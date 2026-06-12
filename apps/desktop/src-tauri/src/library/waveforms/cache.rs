use std::{
    collections::hash_map::DefaultHasher,
    fs,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use super::super::{path_to_string, system_time_ms, LibraryWaveformTile};
use crate::storage::{RecordingRow, RecordingSegmentRow};

const WAVEFORM_TILE_CACHE_VERSION: &str = "v2";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(super) struct CachedWaveformTile {
    pub(super) file_fingerprint: WaveformFileFingerprint,
    pub(super) scale_amplitude: f64,
    pub(super) tile: LibraryWaveformTile,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(super) struct WaveformTrackScaleCache {
    pub(super) file_fingerprints: Vec<WaveformFileFingerprint>,
    pub(super) scale_amplitude: f64,
}

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
        .join(WAVEFORM_TILE_CACHE_VERSION)
        .join(format!("segment-{}", safe_cache_component(&segment.id)))
        .join(format!("stream-{stream_index}"))
        .join(format!("bin-{bin_ms}"))
        .join(format!(
            "tile-{tile_start_ms}-{}.json",
            hash_cache_key(fingerprint)
        ))
}

pub(super) fn waveform_track_scale_cache_path(
    recording: &RecordingRow,
    track_id: &str,
    fingerprints: &[WaveformFileFingerprint],
) -> PathBuf {
    Path::new(&recording.storage_dir)
        .join("waveforms")
        .join(WAVEFORM_TILE_CACHE_VERSION)
        .join(format!("scale-{}", safe_cache_component(track_id)))
        .join(format!("{}.json", hash_cache_key(&fingerprints)))
}

pub(super) fn read_cached_waveform_tile(path: &Path) -> Result<CachedWaveformTile, String> {
    let content = fs::read_to_string(path).map_err(|error| error.to_string())?;
    serde_json::from_str(&content).map_err(|error| error.to_string())
}

pub(super) fn write_cached_waveform_tile(
    path: &Path,
    tile: &CachedWaveformTile,
) -> Result<(), String> {
    write_json_cache(path, tile)
}

pub(super) fn read_waveform_track_scale_cache(
    path: &Path,
) -> Result<WaveformTrackScaleCache, String> {
    let content = fs::read_to_string(path).map_err(|error| error.to_string())?;
    serde_json::from_str(&content).map_err(|error| error.to_string())
}

pub(super) fn write_waveform_track_scale_cache(
    path: &Path,
    cache: &WaveformTrackScaleCache,
) -> Result<(), String> {
    write_json_cache(path, cache)
}

fn write_json_cache<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "Unable to create waveform cache directory {}: {error}",
                path_to_string(parent)
            )
        })?;
    }
    let json = serde_json::to_vec(value).map_err(|error| error.to_string())?;
    fs::write(path, json).map_err(|error| {
        format!(
            "Unable to write waveform cache file {}: {error}",
            path_to_string(path)
        )
    })
}

pub(super) fn waveform_file_fingerprint(path: &str) -> Result<WaveformFileFingerprint, String> {
    let metadata = fs::metadata(path)
        .map_err(|error| format!("Unable to inspect waveform source file {path}: {error}"))?;
    let modified_ms = metadata
        .modified()
        .ok()
        .and_then(system_time_ms)
        .unwrap_or_default();

    Ok(WaveformFileFingerprint {
        modified_ms,
        path: path.to_string(),
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

fn hash_cache_key<T: Hash>(value: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}
