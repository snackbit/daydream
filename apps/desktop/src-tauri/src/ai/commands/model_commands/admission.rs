use super::super::super::*;

pub(in crate::ai::commands) fn admit_model_action<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    kind: &str,
) -> Result<Option<crate::maintenance::runtime::RuntimeActivityLease>, AiError> {
    crate::maintenance::runtime::admit(app, kind).map_err(|message| AiError {
        code: "ai_maintenance_active".into(),
        message,
    })
}

pub(super) fn with_model_admission<R: tauri::Runtime, T>(
    app: &tauri::AppHandle<R>,
    kind: &str,
    operation: impl FnOnce() -> Result<T, AiError>,
) -> Result<T, AiError> {
    let _activity = admit_model_action(app, kind)?;
    operation()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn denied_model_admission_preserves_existing_file_and_sqlite_assets_and_notifications() {
        let registry = crate::maintenance::runtime::RuntimeActivityRegistry::default();
        let app = tauri::test::mock_builder()
            .manage(registry.clone())
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .unwrap();
        let directory =
            std::env::temp_dir().join(format!("daydream-model-admission-{}", Uuid::now_v7()));
        fs::create_dir_all(&directory).unwrap();
        let partial = directory.join("synthetic.part");
        fs::write(&partial, b"prior partial data").unwrap();
        let storage = storage::test_storage(directory.join("test.sqlite3"));
        let catalog = ensure_catalog_model("multilingual-e5-small-q8_0").unwrap();
        let operation = || {
            fs::write(&partial, b"new data").map_err(file_error)?;
            storage
                .with_connection(|connection| {
                    save_catalog_model(
                        connection,
                        catalog,
                        ModelAssetPatch {
                            download_status: "queued".into(),
                            ..ModelAssetPatch::missing()
                        },
                        now_ms(),
                    )?;
                    save_model_download_notification(
                        connection,
                        ModelDownloadNotificationInput {
                            actions: &[model_download_cancel_action(catalog.id)],
                            message: "Synthetic model queued.",
                            model_id: catalog.id,
                            preferred_notification_id: None,
                            status: "running",
                            timestamp_ms: now_ms(),
                            title: "Synthetic model",
                        },
                    )
                    .map(|_| ())
                })
                .map_err(storage_error)
        };
        let maintenance = registry.begin_maintenance().unwrap();
        let error =
            with_model_admission(app.handle(), "model-download-command", operation).unwrap_err();
        assert_eq!(error.code, "ai_maintenance_active");
        assert_eq!(fs::read(&partial).unwrap(), b"prior partial data");
        storage
            .with_connection(|connection| {
                assert!(storage::get_ai_model_asset(connection, catalog.id)?.is_none());
                assert_eq!(
                    connection.query_row("SELECT count(*) FROM notifications", [], |row| row
                        .get::<_, i64>(0))?,
                    0
                );
                Ok(())
            })
            .unwrap();
        drop(maintenance);
        with_model_admission(app.handle(), "model-download-command", operation).unwrap();
        assert_eq!(fs::read(&partial).unwrap(), b"new data");
        assert!(storage
            .with_connection(|connection| storage::get_ai_model_asset(connection, catalog.id))
            .unwrap()
            .is_some());
        fs::remove_dir_all(directory).unwrap();
    }
}
