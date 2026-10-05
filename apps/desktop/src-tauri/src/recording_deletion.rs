//! Filesystem cleanup shared by explicit deletion and retention.
use crate::storage::{self, AppStorage, RecordingRow};
use std::{
    fs::{self, File, OpenOptions},
    path::{Path, PathBuf},
};

pub(crate) struct RecordingDataLease {
    _file: File,
}

/// Recovery and process-local privacy leases require one owner of an app data directory.
pub(crate) fn acquire_recording_data_lease(
    app_data_dir: &Path,
) -> Result<RecordingDataLease, String> {
    fs::create_dir_all(app_data_dir)
        .map_err(|_| "Unable to initialize recording storage.".to_string())?;
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(app_data_dir.join("recording-data.lock"))
        .map_err(|_| "Unable to lock recording storage.".to_string())?;
    file.try_lock().map_err(|_| "Recording storage is already open in another Daydream process. Close that instance and retry.".to_string())?;
    Ok(RecordingDataLease { _file: file })
}

pub(crate) fn remove_processing_artifacts(
    app_data_dir: &Path,
    job_ids: &[String],
) -> Result<(), String> {
    for job_id in job_ids {
        uuid::Uuid::parse_str(job_id)
            .map_err(|_| "Invalid processing job identity.".to_string())?;
        for kind in ["transcription", "visual", "local-transcription"] {
            let path = app_data_dir.join("processing").join(kind).join(job_id);
            if path.exists() {
                fs::remove_dir_all(&path)
                    .map_err(|_| "Unable to remove private processing artifacts.".to_string())?;
            }
        }
    }
    Ok(())
}

/// Called once during startup, before any extraction or transcription worker exists.
pub(crate) fn remove_stale_processing_artifacts(app_data_dir: &Path) -> Result<(), String> {
    let root = app_data_dir.join("processing");
    if root.exists() {
        fs::remove_dir_all(root)
            .map_err(|_| "Unable to remove abandoned processing artifacts.".to_string())?;
    }
    Ok(())
}

/// Caller holds the recording's exclusive mutation lease, after cancelling workers.
pub(crate) fn delete_recording(
    storage: &AppStorage,
    recordings_root: &Path,
    recording: &RecordingRow,
) -> Result<(), String> {
    let jobs = storage
        .with_connection(|connection| {
            storage::list_processing_jobs(connection, Some(&recording.id))
        })
        .map_err(|error| error.to_string())?;
    let app_data_dir = recordings_root
        .parent()
        .ok_or_else(|| "Recordings storage has no parent directory.".to_string())?;
    remove_processing_artifacts(
        app_data_dir,
        &jobs.into_iter().map(|job| job.id).collect::<Vec<_>>(),
    )?;
    let storage_dir = PathBuf::from(&recording.storage_dir);
    if storage_dir.exists() {
        let canonical_root = recordings_root
            .canonicalize()
            .map_err(|_| "Unable to inspect recordings storage.".to_string())?;
        let canonical_path = storage_dir
            .canonicalize()
            .map_err(|_| "Unable to inspect recording storage.".to_string())?;
        if canonical_path == canonical_root || !canonical_path.starts_with(&canonical_root) {
            return Err(
                "Refusing to delete a recording directory outside Daydream storage.".to_string(),
            );
        }
        fs::remove_dir_all(&storage_dir)
            .map_err(|_| "Unable to delete recording media.".to_string())?;
    }
    storage
        .with_connection(|connection| storage::delete_recording(connection, &recording.id))
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn concurrent_startup_cannot_remove_another_process_processing_files() {
        let root =
            std::env::temp_dir().join(format!("daydream-data-lock-{}", uuid::Uuid::now_v7()));
        let first = acquire_recording_data_lease(&root).unwrap();
        assert!(acquire_recording_data_lease(&root).is_err());
        drop(first);
        drop(acquire_recording_data_lease(&root).unwrap());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn deletion_removes_crash_leftovers_for_only_the_owned_job() {
        let root = std::env::temp_dir().join(format!("daydream-deletion-{}", uuid::Uuid::now_v7()));
        let owned = uuid::Uuid::now_v7().to_string();
        let other = uuid::Uuid::now_v7().to_string();
        for job in [&owned, &other] {
            for kind in ["transcription", "visual"] {
                let path = root.join("processing").join(kind).join(job);
                fs::create_dir_all(&path).unwrap();
                fs::write(path.join("private.bin"), b"private").unwrap();
            }
        }
        remove_processing_artifacts(&root, std::slice::from_ref(&owned)).unwrap();
        assert!(!root.join("processing/transcription").join(&owned).exists());
        assert!(!root.join("processing/visual").join(&owned).exists());
        assert!(root.join("processing/visual").join(&other).exists());
        remove_stale_processing_artifacts(&root).unwrap();
        assert!(!root.join("processing").exists());
        fs::remove_dir_all(root).unwrap();
    }
}
