use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::{
    settings::StorageSettings,
    storage::{self, AppStorage, RecordingRow, RecordingSegmentRow},
};

const DAY_MS: i64 = 24 * 60 * 60 * 1_000;

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StorageCleanupPreview {
    pub recording_count: usize,
    pub size_bytes: u64,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StorageCleanupResult {
    pub deleted_recording_count: usize,
    pub deleted_size_bytes: u64,
    pub failed_recording_count: usize,
}

#[derive(Clone, Debug)]
struct CleanupPlan {
    candidates: Vec<CleanupCandidate>,
}

#[derive(Clone, Debug)]
struct CleanupCandidate {
    recording: RecordingRow,
    size_bytes: u64,
}

#[derive(Clone, Debug)]
struct RecordingEntry {
    recording: RecordingRow,
    size_bytes: u64,
}

pub fn preview_storage_cleanup(
    storage: &AppStorage,
    recordings_root: &Path,
    settings: &StorageSettings,
) -> Result<StorageCleanupPreview, String> {
    let plan = storage_cleanup_plan(storage, recordings_root, settings, now_ms())?;

    Ok(StorageCleanupPreview {
        recording_count: plan.candidates.len(),
        size_bytes: plan
            .candidates
            .iter()
            .map(|candidate| candidate.size_bytes)
            .sum(),
    })
}

pub fn enforce_storage_cleanup(
    storage: &AppStorage,
    recordings_root: &Path,
    settings: &StorageSettings,
) -> StorageCleanupResult {
    enforce_storage_cleanup_at(storage, recordings_root, settings, now_ms())
}

pub(crate) fn enforce_storage_cleanup_at(
    storage: &AppStorage,
    recordings_root: &Path,
    settings: &StorageSettings,
    now_ms: i64,
) -> StorageCleanupResult {
    let plan = match storage_cleanup_plan(storage, recordings_root, settings, now_ms) {
        Ok(plan) => plan,
        Err(_error) => {
            warn!("Storage cleanup planning failed.");
            return StorageCleanupResult {
                failed_recording_count: 1,
                ..StorageCleanupResult::default()
            };
        }
    };

    let mut result = StorageCleanupResult::default();
    for candidate in plan.candidates {
        match delete_recording(storage, recordings_root, &candidate.recording) {
            Ok(()) => {
                result.deleted_recording_count += 1;
                result.deleted_size_bytes += candidate.size_bytes;
            }
            Err(_error) => {
                result.failed_recording_count += 1;
                warn!("Storage cleanup failed to delete an eligible recording.");
            }
        }
    }

    result
}

fn storage_cleanup_plan(
    storage: &AppStorage,
    _recordings_root: &Path,
    settings: &StorageSettings,
    now_ms: i64,
) -> Result<CleanupPlan, String> {
    let entries = storage
        .with_connection(|connection| {
            let recordings = storage::list_recordings(connection)?;
            recordings
                .into_iter()
                .map(|recording| {
                    let segments = storage::list_recording_segments(connection, &recording.id)?;
                    Ok(RecordingEntry {
                        size_bytes: recording_size_bytes(&recording, &segments),
                        recording,
                    })
                })
                .collect::<rusqlite::Result<Vec<_>>>()
        })
        .map_err(|error| error.to_string())?;

    let retention_cutoff_ms = now_ms.saturating_sub(i64::from(settings.retention_days) * DAY_MS);
    let mut planned_ids = HashSet::new();
    let mut candidates = entries
        .iter()
        .filter(|entry| is_cleanup_eligible(&entry.recording))
        .filter(|entry| retention_reference_ms(&entry.recording) < retention_cutoff_ms)
        .map(cleanup_candidate)
        .collect::<Vec<_>>();
    candidates.sort_by(cleanup_candidate_order);
    for candidate in &candidates {
        planned_ids.insert(candidate.recording.id.clone());
    }

    let mut retained_size = entries
        .iter()
        .filter(|entry| !planned_ids.contains(&entry.recording.id))
        .map(|entry| entry.size_bytes)
        .sum::<u64>();

    if retained_size > settings.size_limit_bytes {
        let mut cap_candidates = entries
            .iter()
            .filter(|entry| is_cleanup_eligible(&entry.recording))
            .filter(|entry| !planned_ids.contains(&entry.recording.id))
            .map(cleanup_candidate)
            .collect::<Vec<_>>();
        cap_candidates.sort_by(cleanup_candidate_order);

        for candidate in cap_candidates {
            if retained_size <= settings.size_limit_bytes {
                break;
            }
            retained_size = retained_size.saturating_sub(candidate.size_bytes);
            planned_ids.insert(candidate.recording.id.clone());
            candidates.push(candidate);
        }
    }

    Ok(CleanupPlan { candidates })
}

fn cleanup_candidate(entry: &RecordingEntry) -> CleanupCandidate {
    CleanupCandidate {
        recording: entry.recording.clone(),
        size_bytes: entry.size_bytes,
    }
}

fn cleanup_candidate_order(
    left: &CleanupCandidate,
    right: &CleanupCandidate,
) -> std::cmp::Ordering {
    cleanup_order_ms(&left.recording)
        .cmp(&cleanup_order_ms(&right.recording))
        .then_with(|| left.recording.id.cmp(&right.recording.id))
}

fn is_cleanup_eligible(recording: &RecordingRow) -> bool {
    storage::is_deletable_recording_status(&recording.status)
}

fn retention_reference_ms(recording: &RecordingRow) -> i64 {
    recording
        .ended_at_ms
        .or(recording.started_at_ms)
        .unwrap_or(recording.created_at_ms)
}

fn cleanup_order_ms(recording: &RecordingRow) -> i64 {
    retention_reference_ms(recording)
}

fn delete_recording(
    storage: &AppStorage,
    recordings_root: &Path,
    recording: &RecordingRow,
) -> Result<(), String> {
    let storage_dir = PathBuf::from(&recording.storage_dir);

    if storage_dir.exists() {
        ensure_path_inside_recordings_root(recordings_root, &storage_dir)?;
        fs::remove_dir_all(&storage_dir)
            .map_err(|_error| "Unable to delete an eligible recording directory.".to_string())?;
    }

    storage
        .with_connection(|connection| storage::delete_recording(connection, &recording.id))
        .map_err(|_error| "Unable to delete an eligible recording row.".to_string())
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

fn ensure_path_inside_recordings_root(recordings_root: &Path, path: &Path) -> Result<(), String> {
    let canonical_root = recordings_root
        .canonicalize()
        .map_err(|_error| "Unable to inspect recordings root.".to_string())?;
    let canonical_path = path
        .canonicalize()
        .map_err(|_error| "Unable to inspect recording directory.".to_string())?;
    if !canonical_path.starts_with(canonical_root) {
        return Err(
            "Refusing to delete a recording directory outside Daydream storage.".to_string(),
        );
    }

    Ok(())
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use std::fs;

    use uuid::Uuid;

    use crate::storage::{self, CreateRecordingInput};

    use super::*;

    #[test]
    fn preview_selects_expired_completed_and_failed_recordings_only() {
        let app_dir = test_dir("retention-preview");
        let recordings_root = app_dir.join("recordings");
        fs::create_dir_all(&recordings_root).expect("create recordings root");
        let storage = storage::test_storage(app_dir.join("daydream.sqlite3"));
        insert_recording(&storage, &recordings_root, "completed", 1_000, 2_000, 10);
        insert_recording(&storage, &recordings_root, "failed", 2_000, 3_000, 20);
        insert_recording(&storage, &recordings_root, "recording", 1_000, 2_000, 30);
        insert_recording(
            &storage,
            &recordings_root,
            "completed",
            DAY_MS + 20_000,
            DAY_MS + 21_000,
            40,
        );

        let plan = storage_cleanup_plan(
            &storage,
            &recordings_root,
            &StorageSettings {
                retention_days: 1,
                size_limit_bytes: u64::MAX,
            },
            2 * DAY_MS,
        )
        .expect("preview cleanup");

        let _ = fs::remove_dir_all(app_dir);
        assert_eq!(plan.candidates.len(), 2);
        assert_eq!(
            plan.candidates
                .iter()
                .map(|candidate| candidate.size_bytes)
                .sum::<u64>(),
            30
        );
    }

    #[test]
    fn cap_cleanup_deletes_oldest_ended_recordings_first() {
        let app_dir = test_dir("cap-preview");
        let recordings_root = app_dir.join("recordings");
        fs::create_dir_all(&recordings_root).expect("create recordings root");
        let storage = storage::test_storage(app_dir.join("daydream.sqlite3"));
        let oldest = insert_recording(&storage, &recordings_root, "completed", 1_000, 2_000, 30);
        let newest = insert_recording(&storage, &recordings_root, "completed", 5_000, 6_000, 30);
        insert_recording(&storage, &recordings_root, "completed", 3_000, 4_000, 30);

        let plan = storage_cleanup_plan(
            &storage,
            &recordings_root,
            &StorageSettings {
                retention_days: 365,
                size_limit_bytes: 40,
            },
            7_000,
        )
        .expect("preview cleanup");

        let _ = fs::remove_dir_all(app_dir);
        assert_eq!(plan.candidates.len(), 2);
        assert_eq!(plan.candidates[0].recording.id, oldest);
        assert_ne!(plan.candidates[1].recording.id, newest);
    }

    #[test]
    fn enforcement_removes_directories_and_database_rows() {
        let app_dir = test_dir("cleanup-enforcement");
        let recordings_root = app_dir.join("recordings");
        fs::create_dir_all(&recordings_root).expect("create recordings root");
        let storage = storage::test_storage(app_dir.join("daydream.sqlite3"));
        let expired = insert_recording(&storage, &recordings_root, "completed", 1_000, 2_000, 10);
        let retained = insert_recording(
            &storage,
            &recordings_root,
            "completed",
            DAY_MS + 20_000,
            DAY_MS + 21_000,
            20,
        );

        let result = enforce_storage_cleanup_at(
            &storage,
            &recordings_root,
            &StorageSettings {
                retention_days: 1,
                size_limit_bytes: u64::MAX,
            },
            2 * DAY_MS,
        );

        assert_eq!(
            result,
            StorageCleanupResult {
                deleted_recording_count: 1,
                deleted_size_bytes: 10,
                failed_recording_count: 0,
            }
        );
        assert!(storage
            .with_connection(|connection| storage::get_recording(connection, &expired))
            .is_err());
        assert!(storage
            .with_connection(|connection| storage::get_recording(connection, &retained))
            .is_ok());
        assert!(!recordings_root.join(expired).exists());

        let _ = fs::remove_dir_all(app_dir);
    }

    fn insert_recording(
        storage: &AppStorage,
        recordings_root: &Path,
        status: &str,
        started_at_ms: i64,
        ended_at_ms: i64,
        size_bytes: usize,
    ) -> String {
        let id = Uuid::now_v7().to_string();
        let recording_dir = recordings_root.join(&id);
        fs::create_dir_all(&recording_dir).expect("create recording dir");
        fs::write(recording_dir.join("recording.mkv"), vec![1_u8; size_bytes])
            .expect("write recording bytes");

        storage
            .with_connection(|connection| {
                storage::create_recording(
                    connection,
                    &CreateRecordingInput {
                        id: id.clone(),
                        active_scene_id: None,
                        created_at_ms: started_at_ms - 100,
                        default_fps: 0.2,
                        notes: None,
                        output_root: Some(recordings_root.to_string_lossy().to_string()),
                        quality_label: "ocr".to_string(),
                        storage_dir: recording_dir.to_string_lossy().to_string(),
                    },
                )?;
                storage::mark_recording_started(connection, &id, started_at_ms)?;
                match status {
                    "completed" => {
                        storage::mark_recording_completed(connection, &id, ended_at_ms)?;
                    }
                    "failed" => {
                        storage::update_recording_status(connection, &id, "failed")?;
                    }
                    "recording" => {}
                    _ => unreachable!("unsupported test status"),
                }
                Ok(())
            })
            .expect("insert recording");

        id
    }

    fn test_dir(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("daydream-storage-limits-{name}-{}", Uuid::now_v7()))
    }
}
