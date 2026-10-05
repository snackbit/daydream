use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::{json, Value};
use tauri::{Manager, State};
use uuid::Uuid;

use crate::{
    ai, ffmpeg,
    playback_server::PlaybackServer,
    settings,
    storage::{
        self, AppStorage, OcrRegionRow, ProcessingChunkRow, ProcessingJobRow, RecordingRow,
        RecordingSegmentRow, RecordingTrackRow, RedactionRow, TranscriptSegmentRow,
        TranscriptWordRow, VlmObservationRow,
    },
};

const RECORDING_TITLE_MAX_CHARS: usize = 120;

mod waveforms;

mod types;
pub(crate) use types::*;

pub(crate) mod commands;
mod debug;
mod mapping;
mod playback;
mod redactions;
mod thumbnails;
use mapping::*;
use playback::*;
use redactions::*;
use thumbnails::*;

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

pub(crate) fn delete_recording_row(
    storage: &AppStorage,
    recordings_root: &Path,
    recording: &RecordingRow,
) -> Result<(), String> {
    crate::recording_deletion::delete_recording(storage, recordings_root, recording)
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

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

#[cfg(test)]
mod tests;
