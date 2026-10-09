//! Unpublished copies remain ledger-owned until removal or transactional publication.
use crate::storage::{self, AppStorage};
use std::{fs, path::Path};

/// Caller excludes new recovery work (or is in startup before any writer exists).
/// Deletion/retention call this before cascading away the original's ledger.
pub(crate) fn remove_recovery_outputs(
    storage: &AppStorage,
    root: &Path,
    original: Option<&str>,
) -> Result<(), String> {
    let outputs = storage
        .with_connection(|connection| storage::recovery::pending_outputs(connection, original))
        .map_err(|_| "Unable to read pending recovery outputs.")?;
    #[cfg(target_os = "linux")]
    if !outputs.is_empty() && !crate::recorder::media_worker::is_quiescent() {
        return Err("Recovery files are still owned by a media worker. Retry after it stops, or restart Daydream if cleanup is pending.".into());
    }
    for (operation, output_id) in outputs {
        uuid::Uuid::parse_str(&output_id).map_err(|_| "Invalid recovery output identity.")?;
        let published = storage
            .with_connection(|connection| {
                connection.query_row(
                    "SELECT EXISTS(SELECT 1 FROM recordings WHERE id=?1)",
                    [&output_id],
                    |row| row.get::<_, bool>(0),
                )
            })
            .map_err(|_| "Unable to verify recovery output ownership.")?;
        if published {
            return Err("Recovery cleanup cannot remove a published recording.".into());
        }
        let output = root.join(&output_id);
        match fs::symlink_metadata(&output) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err("Unable to inspect pending recovery files.".into()),
            Ok(metadata) => {
                if metadata.file_type().is_symlink() || !metadata.is_dir() {
                    return Err("Recovery output is not an app-owned directory.".into());
                }
                let canonical_root = root
                    .canonicalize()
                    .map_err(|_| "Unable to inspect recordings storage.")?;
                let canonical_output = output
                    .canonicalize()
                    .map_err(|_| "Unable to inspect recovery output.")?;
                if canonical_output.parent() != Some(canonical_root.as_path()) {
                    return Err("Recovery output is outside app-owned recording storage.".into());
                }
                fs::remove_dir_all(&output)
                    .map_err(|_| "Unable to remove unpublished recovery files; retry cleanup.")?;
            }
        }
        storage
            .with_connection(|connection| {
                storage::recovery::output_removed(connection, &operation, &output_id)
            })
            .map_err(|_| "Recovery files were removed but cleanup state could not be saved.")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn owned_output(storage: &AppStorage, root: &Path) -> (storage::RecordingRow, String, String) {
        let original = uuid::Uuid::now_v7().to_string();
        let operation = uuid::Uuid::now_v7().to_string();
        let output = uuid::Uuid::now_v7().to_string();
        let directory = root.join(&original);
        fs::create_dir_all(&directory).unwrap();
        let recording = storage
            .with_connection(|connection| {
                storage::create_recording(
                    connection,
                    &storage::CreateRecordingInput {
                        id: original.clone(),
                        active_scene_id: None,
                        created_at_ms: 1,
                        default_fps: 1.0,
                        notes: None,
                        output_root: None,
                        quality_label: "balanced-ocr".into(),
                        storage_dir: directory.to_string_lossy().into(),
                    },
                )?;
                connection.execute(
                    "UPDATE recordings SET status='failed' WHERE id=?1",
                    [&original],
                )?;
                storage::recovery::start_operation(connection, &operation, &original, &output, 1)?;
                storage::get_recording(connection, &original)
            })
            .unwrap();
        fs::create_dir(root.join(&output)).unwrap();
        fs::write(
            root.join(&output).join("private.mkv"),
            b"synthetic private media",
        )
        .unwrap();
        (recording, operation, output)
    }

    #[test]
    fn deletion_and_startup_cleanup_remove_only_ledger_owned_unpublished_outputs() {
        #[cfg(target_os = "linux")]
        let _serial = crate::recorder::media_worker::PROCESS_TEST.lock().unwrap();
        let app_root = std::env::temp_dir().join(format!(
            "daydream-recovery-ownership-{}",
            uuid::Uuid::now_v7()
        ));
        let storage = storage::test_storage(app_root.clone());
        let root = app_root.join("recordings");
        let (original, _, output) = owned_output(&storage, &root);
        let (other, _, other_output) = owned_output(&storage, &root);
        super::super::delete_recording(&storage, &root, &original).unwrap();
        assert!(!root.join(output).exists());
        assert!(!Path::new(&original.storage_dir).exists());
        assert!(root.join(&other_output).exists());
        assert_eq!(
            storage
                .with_connection(|connection| storage::recovery::pending_outputs(connection, None))
                .unwrap()
                .len(),
            1
        );
        // Startup happens before a writer can exist; interrupted rows are still owned.
        remove_recovery_outputs(&storage, &root, None).unwrap();
        assert!(!root.join(other_output).exists());
        assert!(Path::new(&other.storage_dir).exists());
        assert!(storage
            .with_connection(|connection| storage::recovery::pending_outputs(connection, None))
            .unwrap()
            .is_empty());
        remove_recovery_outputs(&storage, &root, None).unwrap();
        fs::remove_dir_all(app_root).unwrap();
    }

    #[test]
    fn cleanup_commit_failure_retains_durable_ownership_and_retry_handles_already_removed_files() {
        #[cfg(target_os = "linux")]
        let _serial = crate::recorder::media_worker::PROCESS_TEST.lock().unwrap();
        let app_root = std::env::temp_dir().join(format!(
            "daydream-recovery-cleanup-retry-{}",
            uuid::Uuid::now_v7()
        ));
        let storage = storage::test_storage(app_root.clone());
        let root = app_root.join("recordings");
        let (original, operation, output) = owned_output(&storage, &root);
        storage.with_connection(|connection| {
            storage::recovery::fail_operation(connection, &operation)?;
            connection.execute_batch("CREATE TRIGGER reject_cleanup BEFORE UPDATE OF pending_output_id ON recording_recovery_operations BEGIN SELECT RAISE(FAIL,'synthetic cleanup commit failure'); END;")
        }).unwrap();
        assert!(remove_recovery_outputs(&storage, &root, Some(&original.id)).is_err());
        assert!(!root.join(&output).exists());
        assert_eq!(
            storage
                .with_connection(|connection| storage::recovery::pending_outputs(connection, None))
                .unwrap(),
            vec![(operation, output)]
        );
        storage
            .with_connection(|connection| connection.execute_batch("DROP TRIGGER reject_cleanup;"))
            .unwrap();
        remove_recovery_outputs(&storage, &root, None).unwrap();
        assert!(storage
            .with_connection(|connection| storage::recovery::pending_outputs(connection, None))
            .unwrap()
            .is_empty());
        fs::remove_dir_all(app_root).unwrap();
    }

    #[test]
    fn cleanup_refuses_published_outputs_and_preserves_their_ledger() {
        #[cfg(target_os = "linux")]
        let _serial = crate::recorder::media_worker::PROCESS_TEST.lock().unwrap();
        let app_root = std::env::temp_dir().join(format!(
            "daydream-recovery-published-{}",
            uuid::Uuid::now_v7()
        ));
        let storage = storage::test_storage(app_root.clone());
        let root = app_root.join("recordings");
        let (_, _, output) = owned_output(&storage, &root);
        storage
            .with_connection(|connection| {
                storage::create_recording(
                    connection,
                    &storage::CreateRecordingInput {
                        id: output.clone(),
                        active_scene_id: None,
                        created_at_ms: 1,
                        default_fps: 1.0,
                        notes: None,
                        output_root: None,
                        quality_label: "balanced-ocr".into(),
                        storage_dir: root.join(&output).to_string_lossy().into(),
                    },
                )
            })
            .unwrap();
        assert!(remove_recovery_outputs(&storage, &root, None).is_err());
        assert!(root.join(output).join("private.mkv").exists());
        assert_eq!(
            storage
                .with_connection(|connection| storage::recovery::pending_outputs(connection, None))
                .unwrap()
                .len(),
            1
        );
        fs::remove_dir_all(app_root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn cleanup_rejects_symlinks_without_removing_outside_files_or_forgetting_ownership() {
        #[cfg(target_os = "linux")]
        let _serial = crate::recorder::media_worker::PROCESS_TEST.lock().unwrap();
        let app_root = std::env::temp_dir().join(format!(
            "daydream-recovery-symlink-{}",
            uuid::Uuid::now_v7()
        ));
        let storage = storage::test_storage(app_root.clone());
        let root = app_root.join("recordings");
        let (_, _, output) = owned_output(&storage, &root);
        fs::remove_dir_all(root.join(&output)).unwrap();
        let outside = app_root.join("outside");
        fs::create_dir(&outside).unwrap();
        fs::write(outside.join("private.mkv"), b"synthetic outside media").unwrap();
        std::os::unix::fs::symlink(&outside, root.join(&output)).unwrap();
        assert!(remove_recovery_outputs(&storage, &root, None).is_err());
        assert!(outside.join("private.mkv").exists());
        assert_eq!(
            storage
                .with_connection(|connection| storage::recovery::pending_outputs(connection, None))
                .unwrap()
                .len(),
            1
        );
        fs::remove_file(root.join(output)).unwrap();
        remove_recovery_outputs(&storage, &root, None).unwrap();
        fs::remove_dir_all(app_root).unwrap();
    }
}
