use super::super::*;
use super::transfer_state::save_stopped_download;
use super::{
    checksum_file, delete_file_if_exists, downloaded_bytes, emit_model_list, is_whisper_model,
    rollback_finalized_model_file, save_catalog_model, CatalogModel, ModelAssetPatch,
};

pub(in crate::ai) async fn finalize_complete_partial_if_valid(
    app: &AppHandle,
    catalog: &CatalogModel,
    partial_path: &Path,
    final_path: &Path,
    manager: &AiModelManager,
) -> Result<bool, AiError> {
    if !partial_path.is_file() {
        return Ok(false);
    }
    if checksum_file(partial_path, catalog)? != catalog.sha1 {
        delete_file_if_exists(partial_path)?;
        return Ok(false);
    }
    finalize_model_download(app, catalog, partial_path, final_path, manager).await?;
    Ok(true)
}

pub(in crate::ai) async fn finalize_model_download(
    app: &AppHandle,
    catalog: &CatalogModel,
    partial_path: &Path,
    final_path: &Path,
    manager: &AiModelManager,
) -> Result<(), AiError> {
    let storage = app.state::<AppStorage>();
    manager.set_telemetry(catalog.id, "verifying", None, None);
    emit_model_list(app);
    if manager.is_cancelled(catalog.id) || manager.is_paused(catalog.id) {
        save_stopped_download(app, catalog, downloaded_bytes(partial_path, final_path))?;
        return Ok(());
    }
    let actual_sha1 = checksum_file(partial_path, catalog)?;
    if actual_sha1 != catalog.sha1 {
        return Err(AiError {
            code: checksum_failed_code(catalog).to_string(),
            message: format!("Checksum verification failed for {}.", catalog.label),
        });
    }
    if manager.is_cancelled(catalog.id) || manager.is_paused(catalog.id) {
        save_stopped_download(app, catalog, downloaded_bytes(partial_path, final_path))?;
        return Ok(());
    }
    fs::rename(partial_path, final_path).map_err(file_error)?;
    if manager.is_cancelled(catalog.id) || manager.is_paused(catalog.id) {
        save_cancelled_finalized_download(app, catalog, partial_path, final_path)?;
        return Ok(());
    }
    let local_path = final_path.to_string_lossy().to_string();
    manager.set_telemetry(catalog.id, "testing", None, None);
    emit_model_list(app);
    let self_test_error = if is_whisper_model(catalog) {
        let app_settings = settings::load_app_settings(&storage).map_err(settings_error)?;
        let warmup = LocalTranscriptionWarmup {
            local_backend: app_settings.ai.transcription.local_backend,
            model_id: catalog.id.to_string(),
            model_path: final_path.to_path_buf(),
        };
        let provider = app.state::<TranscriptionProviderManager>().inner().clone();
        match tauri::async_runtime::spawn_blocking(move || provider.warm_local_model(warmup)).await
        {
            Ok(Ok(())) => None,
            Ok(Err(error)) => Some(error.code),
            Err(_error) => Some("ai_model_self_test_failed".to_string()),
        }
    } else if catalog.provider == "local-embedding" {
        let mut settings = settings::load_app_settings(&storage)
            .map_err(settings_error)?
            .ai
            .recall
            .embedding;
        settings.provider = "local".into();
        settings.local_backend = "cpu".into();
        settings.local_model_id = catalog.id.into();
        let worker = app
            .path()
            .resource_dir()
            .ok()
            .map(|path| {
                path.join("embedding").join(if cfg!(windows) {
                    "daydream-embedding-worker.exe"
                } else {
                    "daydream-embedding-worker"
                })
            })
            .filter(|path| path.is_file())
            .unwrap_or_else(|| PathBuf::from(env!("DAYDREAM_EMBEDDING_WORKER")));
        let models_dir = final_path
            .parent()
            .and_then(Path::parent)
            .ok_or_else(|| AiError {
                code: "ai_model_cache_unavailable".into(),
                message: "Embedding model cache is unavailable.".into(),
            })?
            .to_path_buf();
        crate::recall::embedding::test_embedding_provider(&settings, models_dir, worker, None)
            .await
            .err()
    } else {
        None
    };
    if manager.is_cancelled(catalog.id) || manager.is_paused(catalog.id) {
        save_cancelled_finalized_download(app, catalog, partial_path, final_path)?;
        return Ok(());
    }
    storage
        .with_connection(|connection| {
            let timestamp_ms = now_ms();
            let tested = is_whisper_model(catalog) || catalog.provider == "local-embedding";
            let self_test_status = if !tested {
                "not-run"
            } else if self_test_error.is_some() {
                "failed"
            } else {
                "passed"
            };
            let notification_status = if self_test_error.is_some() {
                "warning"
            } else {
                "succeeded"
            };
            let notification_title = if !tested {
                "AI asset download complete"
            } else if self_test_error.is_some() {
                "Model download complete; load test failed"
            } else {
                "Model download complete"
            };
            let notification_message = if !tested {
                format!("{} downloaded and verified.", catalog.label)
            } else if let Some(error_code) = self_test_error.as_deref() {
                format!(
                    "{} downloaded and verified, but load test failed: {}.",
                    catalog.label, error_code
                )
            } else {
                format!("{} downloaded, verified, and load-tested.", catalog.label)
            };
            save_catalog_model(
                connection,
                catalog,
                ModelAssetPatch {
                    download_status: "available".into(),
                    downloaded_bytes: catalog.size_bytes,
                    error_code: self_test_error.as_deref().map(Into::into),
                    local_path: Some(local_path.as_str().into()),
                    self_test_status: self_test_status.into(),
                    total_bytes: Some(catalog.size_bytes),
                    verification_status: "passed".into(),
                    ..ModelAssetPatch::from_existing(connection, catalog)?
                },
                timestamp_ms,
            )?;
            save_model_download_notification(
                connection,
                ModelDownloadNotificationInput {
                    actions: &[],
                    message: &notification_message,
                    model_id: catalog.id,
                    preferred_notification_id: None,
                    status: notification_status,
                    timestamp_ms,
                    title: notification_title,
                },
            )
            .map(|_row| ())
        })
        .map_err(storage_error)?;
    manager.set_telemetry(
        catalog.id,
        if self_test_error.is_some() {
            "failed"
        } else {
            "ready"
        },
        None,
        None,
    );
    emit_model_list(app);

    Ok(())
}

fn checksum_failed_code(catalog: &CatalogModel) -> &'static str {
    if catalog.provider == "local-tesseract-ocr" {
        "ai_ocr_tesseract_checksum_failed"
    } else {
        "ai_model_checksum_failed"
    }
}

pub(in crate::ai) fn save_cancelled_finalized_download(
    app: &AppHandle,
    catalog: &CatalogModel,
    partial_path: &Path,
    final_path: &Path,
) -> Result<(), AiError> {
    rollback_finalized_model_file(partial_path, final_path)?;
    save_stopped_download(app, catalog, downloaded_bytes(partial_path, final_path))
}
