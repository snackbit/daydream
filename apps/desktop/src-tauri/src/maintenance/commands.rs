use super::{files, runtime::RuntimeActivityRegistry, *};
use crate::storage::AppStorage;
use rusqlite::OptionalExtension;
use sha2::{Digest, Sha256};
use std::{collections::HashMap, sync::Mutex};
use tauri::{Manager, State};

#[derive(Default)]
pub(crate) struct MaintenanceOwner {
    running: tokio::sync::Mutex<()>,
    previews: Mutex<HashMap<String, MaintenancePreview>>,
}
fn pending(storage: &AppStorage) -> Result<Option<MaintenanceStatus>, String> {
    storage.with_connection(|connection| connection.query_row("SELECT id,action,phase,error FROM maintenance_operations WHERE phase!='completed' ORDER BY created_at_ms DESC LIMIT 1", [], |row| {
        let action:String=row.get(1)?;
        let action=match action.as_str() { "reset-preferences"=>MaintenanceAction::ResetPreferences,"clear-library"=>MaintenanceAction::ClearLibrary,"factory-reset"=>MaintenanceAction::FactoryReset,_=>return Err(rusqlite::Error::InvalidQuery) };
        Ok(MaintenanceStatus{operation_id:row.get(0)?, action, phase:row.get(2)?,error:row.get(3)?})
    }).optional()).map_err(|_|"Unable to read maintenance recovery state.".into())
}
pub(crate) fn initialize(
    storage: &AppStorage,
    registry: &RuntimeActivityRegistry,
) -> Result<MaintenanceOwner, String> {
    if pending(storage)?.is_some() {
        registry.block_for_recovery();
    }
    Ok(MaintenanceOwner::default())
}
fn phase(
    storage: &AppStorage,
    operation: &mut MaintenanceStatus,
    value: &str,
    error: Option<String>,
) -> Result<(), String> {
    storage
        .with_connection(|connection| {
            connection.execute(
                "UPDATE maintenance_operations SET phase=?2,error=?3 WHERE id=?1",
                rusqlite::params![operation.operation_id, value, error],
            )
        })
        .map_err(|_| {
            "Unable to persist maintenance progress. Restart and retry cleanup.".to_string()
        })?;
    operation.phase = value.into();
    operation.error = error;
    Ok(())
}
fn snapshot(
    app: &tauri::AppHandle,
    storage: &AppStorage,
    action: MaintenanceAction,
) -> Result<MaintenancePreview, String> {
    let root = app
        .path()
        .app_data_dir()
        .map_err(|_| "App data directory is unavailable.")?;
    let logs = app
        .path()
        .app_log_dir()
        .unwrap_or_else(|_| root.join("logs"));
    let (file_count, size_bytes, fingerprint) =
        files::inventory(&files::roots(&root, Some(&logs), action))?;
    let (recording_count, library_row_count, model_count) = storage
        .with_connection(crate::storage::maintenance::library_counts)
        .map_err(|_| "Unable to inspect library impact.".to_string())?;
    let mut hash = Sha256::new();
    hash.update(action.key());
    hash.update(fingerprint);
    hash.update(recording_count.to_le_bytes());
    hash.update(library_row_count.to_le_bytes());
    hash.update(model_count.to_le_bytes());
    Ok(MaintenancePreview {
        action,
        token: hex::encode(hash.finalize()),
        recording_count,
        library_row_count,
        model_count,
        file_count,
        size_bytes,
    })
}
#[tauri::command(rename = "previewMaintenance")]
pub(crate) fn preview_maintenance(
    app: tauri::AppHandle,
    input: PreviewMaintenanceInput,
    storage: State<'_, AppStorage>,
    owner: State<'_, MaintenanceOwner>,
) -> Result<MaintenancePreview, String> {
    if let Some(pending) = pending(&storage)? {
        if pending.action != input.action {
            return Err(
                "Finish the pending maintenance operation before choosing another action.".into(),
            );
        }
    }
    let preview = snapshot(&app, &storage, input.action)?;
    owner
        .previews
        .lock()
        .map_err(|_| "Maintenance preview state is unavailable.")?
        .insert(preview.token.clone(), preview.clone());
    Ok(preview)
}
#[tauri::command(rename = "getMaintenanceStatus")]
pub(crate) fn get_maintenance_status(
    storage: State<'_, AppStorage>,
) -> Result<Option<MaintenanceStatus>, String> {
    pending(&storage)
}
#[tauri::command(rename = "executeMaintenance")]
pub(crate) async fn execute_maintenance(
    app: tauri::AppHandle,
    input: ExecuteMaintenanceInput,
    storage: State<'_, AppStorage>,
    owner: State<'_, MaintenanceOwner>,
    registry: State<'_, RuntimeActivityRegistry>,
) -> Result<MaintenanceStatus, String> {
    let _running = owner
        .running
        .try_lock()
        .map_err(|_| "Maintenance is already running.")?;
    if input.confirmation != input.action.confirmation() {
        return Err("The confirmation does not match the selected action.".into());
    }
    let previous = pending(&storage)?;
    if previous
        .as_ref()
        .is_some_and(|operation| operation.action != input.action)
    {
        return Err("Finish the pending maintenance operation first.".into());
    }
    let preview = owner
        .previews
        .lock()
        .map_err(|_| "Maintenance preview state is unavailable.")?
        .get(&input.preview_token)
        .cloned()
        .ok_or("Refresh the maintenance impact before confirming.")?;
    if preview.action != input.action {
        return Err("The preview is for a different action.".into());
    }
    let mut lease = if previous.is_some() {
        registry.resume_maintenance()?
    } else {
        registry.begin_maintenance()?
    };
    if previous.is_some() {
        lease.protect_recovery();
    }
    // A capture writer must never be stopped implicitly by destructive maintenance.
    super::quiesce::ensure_capture_stopped(&app)?;
    if let Err(error) = super::quiesce::quiesce(&app).await {
        if previous.is_some() {
            lease.keep_closed();
        }
        return Err(error);
    }
    if let Err(error) = lease.cancel_and_drain().await {
        if previous.is_some() {
            lease.keep_closed();
        }
        return Err(error);
    }
    if let Err(error) = super::quiesce::release_models(&app).await {
        if previous.is_some() {
            lease.keep_closed();
        }
        return Err(error);
    }
    let current = snapshot(&app, &storage, input.action)?;
    if current.token != input.preview_token {
        if previous.is_some() {
            lease.keep_closed();
        }
        return Err("The cleanup impact changed. Review a fresh preview and confirm again.".into());
    }
    let mut operation = match previous {
        Some(operation) => operation,
        None => {
            let operation = MaintenanceStatus {
                operation_id: uuid::Uuid::now_v7().to_string(),
                action: input.action,
                phase: "files".into(),
                error: None,
            };
            storage.with_connection(|connection|connection.execute("INSERT INTO maintenance_operations(id,action,phase,created_at_ms) VALUES(?1,?2,'files',?3)",rusqlite::params![operation.operation_id,input.action.key(),chrono::Utc::now().timestamp_millis()])).map_err(|_|"Unable to persist maintenance recovery marker.".to_string())?;
            operation
        }
    };
    lease.protect_recovery();
    let result = perform_cleanup(&app, &storage, &mut operation);
    if let Err(error) = result {
        let _ = phase(&storage, &mut operation, "failed", Some(error.clone()));
        lease.keep_closed();
        return Err(error);
    }
    if input.action != MaintenanceAction::ClearLibrary {
        lease.keep_closed();
    } else {
        lease.release();
        crate::startup::resume_after_maintenance(&app);
    } // UI must acknowledge browser theme reset.
    owner
        .previews
        .lock()
        .map_err(|_| "Maintenance preview state is unavailable.")?
        .clear();
    crate::recall::mutations::emit_changed(&app, None);
    Ok(operation)
}
fn perform_cleanup(
    app: &tauri::AppHandle,
    storage: &AppStorage,
    operation: &mut MaintenanceStatus,
) -> Result<(), String> {
    let root = app
        .path()
        .app_data_dir()
        .map_err(|_| "App data directory is unavailable.")?;
    let logs = app
        .path()
        .app_log_dir()
        .unwrap_or_else(|_| root.join("logs"));
    if operation.action == MaintenanceAction::FactoryReset {
        crate::telemetry::close_file_for_factory_reset()?;
    }
    phase(storage, operation, "files", None)?;
    files::remove_roots(&files::roots(&root, Some(&logs), operation.action))?;
    phase(storage, operation, "database", None)?;
    storage
        .with_connection_mut(|connection| {
            if operation.action != MaintenanceAction::ResetPreferences {
                connection.pragma_update(None, "secure_delete", "ON")?;
            }
            let transaction = connection.transaction()?;
            if operation.action != MaintenanceAction::ResetPreferences {
                crate::storage::maintenance::clear_library(
                    &transaction,
                    operation.action == MaintenanceAction::FactoryReset,
                )?;
            }
            if operation.action != MaintenanceAction::ClearLibrary {
                crate::storage::maintenance::reset_preferences(&transaction)?;
            }
            if operation.action == MaintenanceAction::ResetPreferences {
                crate::storage::maintenance::pause_retention_after_reset(&transaction)?;
            }
            transaction.commit()
        })
        .map_err(|_| {
            "Files were removed, but database cleanup failed. Retry maintenance to finish."
                .to_string()
        })?;
    if operation.action != MaintenanceAction::ResetPreferences {
        phase(storage, operation, "compact-database", None)?;
        storage.with_connection(crate::storage::maintenance::compact_after_cleanup).map_err(|_|"Library rows were cleared, but private database pages could not be compacted. Retry cleanup to finish.".to_string())?;
    }
    if operation.action == MaintenanceAction::FactoryReset {
        crate::secret_crypto::delete_platform_settings_key().map_err(|_| "App files and settings were cleared, but the settings encryption key could not be removed. Retry factory reset.".to_string())?;
    }
    if operation.action == MaintenanceAction::ResetPreferences {
        app.state::<crate::recorder::RecorderManager>().reset_preferences_cache().map_err(|_|"Preferences were reset, but the old capture setup could not be cleared safely. Restart Daydream and retry the pending preference reset.".to_string())?;
    }
    phase(
        storage,
        operation,
        if operation.action == MaintenanceAction::ClearLibrary {
            "completed"
        } else {
            "awaiting-theme-reset"
        },
        None,
    )
}
#[tauri::command(rename = "acknowledgeMaintenanceThemeReset")]
pub(crate) fn acknowledge_maintenance_theme_reset(
    app: tauri::AppHandle,
    input: AcknowledgeMaintenanceThemeResetInput,
    storage: State<'_, AppStorage>,
    owner: State<'_, MaintenanceOwner>,
    registry: State<'_, RuntimeActivityRegistry>,
) -> Result<MaintenanceStatus, String> {
    let _running = owner
        .running
        .try_lock()
        .map_err(|_| "Maintenance is already running.")?;
    let mut operation = pending(&storage)?.ok_or("No theme reset is pending.")?;
    if operation.operation_id != input.operation_id || operation.phase != "awaiting-theme-reset" {
        return Err("The theme reset acknowledgement is stale.".into());
    }
    let mut lease = registry.resume_maintenance()?;
    lease.protect_recovery();
    phase(&storage, &mut operation, "completed", None)?;
    if operation.action == MaintenanceAction::FactoryReset {
        lease.keep_closed();
        app.restart();
    }
    lease.release();
    crate::startup::resume_after_maintenance(&app);
    Ok(operation)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn interrupted_cleanup_blocks_admission_after_restart_until_completion() {
        let root = std::env::temp_dir().join(format!(
            "daydream-maintenance-restart-{}",
            uuid::Uuid::now_v7()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("daydream.sqlite3");
        let storage = crate::storage::test_file_storage(path.clone());
        storage.with_connection(|connection| connection.execute("INSERT INTO maintenance_operations(id,action,phase,error,created_at_ms) VALUES('synthetic-operation','clear-library','failed','synthetic cleanup failure',1)",[])).unwrap();
        drop(storage);
        let storage = crate::storage::test_file_storage(path);
        let registry = RuntimeActivityRegistry::default();
        let _owner = initialize(&storage, &registry).unwrap();
        assert!(registry.begin("capture").is_err());
        assert!(registry.begin("playback").is_err());
        let mut operation = pending(&storage).unwrap().unwrap();
        assert_eq!(operation.phase, "failed");
        let mut interrupted_retry = registry.resume_maintenance().unwrap();
        interrupted_retry.protect_recovery();
        drop(interrupted_retry);
        assert!(
            registry.begin("capture").is_err(),
            "Failed retry keeps durable cleanup admission closed"
        );
        let mut exclusive = registry.resume_maintenance().unwrap();
        exclusive.protect_recovery();
        phase(&storage, &mut operation, "completed", None).unwrap();
        exclusive.release();
        assert!(pending(&storage).unwrap().is_none());
        assert!(registry.begin("capture").is_ok());
        drop(storage);
        std::fs::remove_dir_all(root).unwrap();
    }
}
