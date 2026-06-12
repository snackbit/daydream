use super::super::*;
use super::{
    catalog_model, delete_file_if_exists, downloaded_bytes, emit_model_list, ensure_catalog_model,
    model_path, model_source_url, partial_model_path, rollback_finalized_model_file,
    save_catalog_model, sha1_file, CatalogModel, ModelAssetPatch,
};

pub(in crate::ai) fn spawn_model_download(app: AppHandle, model_id: String) {
    tauri::async_runtime::spawn(async move {
        let result = run_model_download(&app, &model_id).await;
        if let Err(error) = result {
            let storage = app.state::<AppStorage>();
            if let Some(catalog) = catalog_model(&model_id) {
                let _ = storage.with_connection(|connection| {
                    let timestamp_ms = now_ms();
                    save_catalog_model(
                        connection,
                        catalog,
                        ModelAssetPatch {
                            download_status: "failed".into(),
                            error_code: Some(error.code.as_str().into()),
                            verification_status: "failed".into(),
                            ..ModelAssetPatch::from_existing(connection, catalog)?
                        },
                        timestamp_ms,
                    )?;
                    save_model_download_notification(
                        connection,
                        ModelDownloadNotificationInput {
                            actions: &[model_download_retry_action(catalog.id)],
                            message: &format!("{} download failed: {}.", catalog.label, error.code),
                            model_id: catalog.id,
                            preferred_notification_id: None,
                            status: "failed",
                            timestamp_ms,
                            title: "Model download failed",
                        },
                    )
                    .map(|_row| ())
                });
                emit_model_list(&app);
            }
        }

        let manager = app.state::<AiModelManager>();
        if let Some(next_model_id) = manager.finish_download(&model_id) {
            spawn_model_download(app.clone(), next_model_id);
        }
    });
}

pub(in crate::ai) async fn run_model_download(
    app: &AppHandle,
    model_id: &str,
) -> Result<(), AiError> {
    let catalog = ensure_catalog_model(model_id)?;
    let storage = app.state::<AppStorage>();
    let manager = app.state::<AiModelManager>();
    let final_path = model_path(app, catalog)?;
    let partial_path = partial_model_path(app, catalog)?;
    if let Some(parent) = final_path.parent() {
        fs::create_dir_all(parent).map_err(file_error)?;
    }

    let existing = storage
        .with_connection(|connection| storage::get_ai_model_asset(connection, catalog.id))
        .map_err(storage_error)?;
    let mut partial_len = match prepare_partial_download(catalog, &partial_path)? {
        PartialDownloadResume::Complete => {
            finalize_model_download(app, catalog, &partial_path, &final_path, &manager).await?;
            return Ok(());
        }
        PartialDownloadResume::Resume(length) => length,
    };
    let client = reqwest::Client::new();
    let mut retried_without_range = false;

    loop {
        let mut request = client.get(model_source_url(catalog));
        if partial_len > 0 {
            request = request.header(RANGE, format!("bytes={partial_len}-"));
            if let Some(etag) = existing.as_ref().and_then(|row| row.etag.clone()) {
                request = request.header(IF_RANGE, etag);
            }
        }
        let response = request.send().await.map_err(download_error)?;
        let status = response.status();
        if status == StatusCode::RANGE_NOT_SATISFIABLE && partial_len > 0 {
            if finalize_complete_partial_if_valid(
                app,
                catalog,
                &partial_path,
                &final_path,
                &manager,
            )
            .await?
            {
                return Ok(());
            }
            if retried_without_range {
                return Err(http_error(status));
            }
            partial_len = 0;
            retried_without_range = true;
            continue;
        }
        if !status.is_success() {
            return Err(http_error(status));
        }
        let append = status == StatusCode::PARTIAL_CONTENT && partial_len > 0;
        if !append {
            partial_len = 0;
        }
        let etag = response
            .headers()
            .get(ETAG)
            .and_then(|value| value.to_str().ok())
            .map(ToOwned::to_owned);
        let response_len = response
            .headers()
            .get(CONTENT_LENGTH)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<i64>().ok());
        let total_bytes = response_len.map(|length| length + partial_len as i64);

        storage
            .with_connection(|connection| {
                save_catalog_model(
                    connection,
                    catalog,
                    ModelAssetPatch {
                        download_status: "downloading".into(),
                        downloaded_bytes: partial_len as i64,
                        etag: etag.as_deref().map(Into::into),
                        total_bytes: total_bytes.or(Some(catalog.size_bytes)),
                        verification_status: "pending".into(),
                        ..ModelAssetPatch::from_existing(connection, catalog)?
                    },
                    now_ms(),
                )
                .map(|_row| ())
            })
            .map_err(storage_error)?;
        emit_model_list(app);

        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .append(append)
            .truncate(!append)
            .open(&partial_path)
            .map_err(file_error)?;
        let mut downloaded = partial_len as i64;
        let mut last_progress_update = Instant::now();
        let mut last_progress_bytes = downloaded;
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            if manager.is_cancelled(catalog.id) {
                save_cancelled_download(app, catalog, downloaded)?;
                return Ok(());
            }
            let chunk = chunk.map_err(download_error)?;
            file.write_all(&chunk).map_err(file_error)?;
            downloaded += chunk.len() as i64;
            if downloaded - last_progress_bytes >= MODEL_PROGRESS_UPDATE_BYTES
                || last_progress_update.elapsed() >= MODEL_PROGRESS_UPDATE_INTERVAL
            {
                save_download_progress(app, catalog, downloaded, total_bytes)?;
                last_progress_update = Instant::now();
                last_progress_bytes = downloaded;
            }
        }
        if downloaded != last_progress_bytes {
            save_download_progress(app, catalog, downloaded, total_bytes)?;
        }
        file.flush().map_err(file_error)?;
        drop(file);
        break;
    }

    finalize_model_download(app, catalog, &partial_path, &final_path, &manager).await
}

#[derive(Debug, PartialEq, Eq)]
pub(in crate::ai) enum PartialDownloadResume {
    Complete,
    Resume(u64),
}

pub(in crate::ai) fn prepare_partial_download(
    catalog: &CatalogModel,
    partial_path: &Path,
) -> Result<PartialDownloadResume, AiError> {
    let partial_len = fs::metadata(partial_path)
        .map(|metadata| metadata.len())
        .unwrap_or(0);
    if partial_len == 0 {
        return Ok(PartialDownloadResume::Resume(0));
    }

    if partial_len >= catalog.size_bytes as u64 {
        if sha1_file(partial_path)? == catalog.sha1 {
            return Ok(PartialDownloadResume::Complete);
        }
        delete_file_if_exists(partial_path)?;
        return Ok(PartialDownloadResume::Resume(0));
    }

    Ok(PartialDownloadResume::Resume(partial_len))
}

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
    if sha1_file(partial_path)? != catalog.sha1 {
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
    if manager.is_cancelled(catalog.id) {
        save_cancelled_download(app, catalog, downloaded_bytes(partial_path, final_path))?;
        return Ok(());
    }
    let actual_sha1 = sha1_file(partial_path)?;
    if actual_sha1 != catalog.sha1 {
        return Err(AiError {
            code: "ai_model_checksum_failed".to_string(),
            message: format!("Model checksum verification failed for {}.", catalog.label),
        });
    }
    if manager.is_cancelled(catalog.id) {
        save_cancelled_download(app, catalog, downloaded_bytes(partial_path, final_path))?;
        return Ok(());
    }
    fs::rename(partial_path, final_path).map_err(file_error)?;
    if manager.is_cancelled(catalog.id) {
        save_cancelled_finalized_download(app, catalog, partial_path, final_path)?;
        return Ok(());
    }
    let local_path = final_path.to_string_lossy().to_string();
    let app_settings = settings::load_app_settings(&storage).map_err(settings_error)?;
    let warmup = LocalTranscriptionWarmup {
        local_backend: app_settings.ai.transcription.local_backend,
        model_id: catalog.id.to_string(),
        model_path: final_path.to_path_buf(),
    };
    let provider = app.state::<TranscriptionProviderManager>().inner().clone();
    let self_test_error =
        match tauri::async_runtime::spawn_blocking(move || provider.warm_local_model(warmup)).await
        {
            Ok(Ok(())) => None,
            Ok(Err(error)) => Some(error.code),
            Err(_error) => Some("ai_model_self_test_failed".to_string()),
        };
    if manager.is_cancelled(catalog.id) {
        save_cancelled_finalized_download(app, catalog, partial_path, final_path)?;
        return Ok(());
    }
    storage
        .with_connection(|connection| {
            let timestamp_ms = now_ms();
            let self_test_status = if self_test_error.is_some() {
                "failed"
            } else {
                "passed"
            };
            let notification_status = if self_test_error.is_some() {
                "warning"
            } else {
                "succeeded"
            };
            let notification_title = if self_test_error.is_some() {
                "Model download complete; load test failed"
            } else {
                "Model download complete"
            };
            let notification_message = if let Some(error_code) = self_test_error.as_deref() {
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
    emit_model_list(app);

    Ok(())
}

pub(in crate::ai) fn save_cancelled_finalized_download(
    app: &AppHandle,
    catalog: &CatalogModel,
    partial_path: &Path,
    final_path: &Path,
) -> Result<(), AiError> {
    rollback_finalized_model_file(partial_path, final_path)?;
    save_cancelled_download(app, catalog, downloaded_bytes(partial_path, final_path))
}

pub(in crate::ai) fn save_download_progress(
    app: &AppHandle,
    catalog: &CatalogModel,
    downloaded: i64,
    total_bytes: Option<i64>,
) -> Result<(), AiError> {
    let storage = app.state::<AppStorage>();
    storage
        .with_connection(|connection| {
            save_catalog_model(
                connection,
                catalog,
                ModelAssetPatch {
                    download_status: "downloading".into(),
                    downloaded_bytes: downloaded,
                    total_bytes: total_bytes.or(Some(catalog.size_bytes)),
                    verification_status: "pending".into(),
                    ..ModelAssetPatch::from_existing(connection, catalog)?
                },
                now_ms(),
            )
            .map(|_row| ())
        })
        .map_err(storage_error)?;
    emit_model_list(app);

    Ok(())
}

pub(in crate::ai) fn save_cancelled_download(
    app: &AppHandle,
    catalog: &CatalogModel,
    downloaded: i64,
) -> Result<(), AiError> {
    let storage = app.state::<AppStorage>();
    storage
        .with_connection(|connection| {
            let timestamp_ms = now_ms();
            save_catalog_model(
                connection,
                catalog,
                ModelAssetPatch {
                    download_status: "cancelled".into(),
                    downloaded_bytes: downloaded,
                    error_code: Some("ai_model_download_cancelled".into()),
                    verification_status: "unknown".into(),
                    ..ModelAssetPatch::from_existing(connection, catalog)?
                },
                timestamp_ms,
            )?;
            save_model_download_notification(
                connection,
                ModelDownloadNotificationInput {
                    actions: &[model_download_retry_action(catalog.id)],
                    message: &format!("{} download was cancelled.", catalog.label),
                    model_id: catalog.id,
                    preferred_notification_id: None,
                    status: "cancelled",
                    timestamp_ms,
                    title: "Model download cancelled",
                },
            )
            .map(|_row| ())
        })
        .map_err(storage_error)?;
    emit_model_list(app);

    Ok(())
}

pub(in crate::ai) fn save_cancel_download_result(
    connection: &rusqlite::Connection,
    catalog: &CatalogModel,
    preferred_notification_id: Option<&str>,
    cancelled: bool,
    downloaded: Option<i64>,
    timestamp_ms: i64,
) -> rusqlite::Result<NotificationRow> {
    if cancelled {
        let existing = ModelAssetPatch::from_existing(connection, catalog)?;
        save_catalog_model(
            connection,
            catalog,
            ModelAssetPatch {
                download_status: "cancelled".into(),
                downloaded_bytes: downloaded.unwrap_or(existing.downloaded_bytes),
                error_code: Some("ai_model_download_cancelled".into()),
                verification_status: "unknown".into(),
                ..existing
            },
            timestamp_ms,
        )?;
        save_model_download_notification(
            connection,
            ModelDownloadNotificationInput {
                actions: &[model_download_retry_action(catalog.id)],
                message: &format!("{} download was cancelled.", catalog.label),
                model_id: catalog.id,
                preferred_notification_id,
                status: "cancelled",
                timestamp_ms,
                title: "Model download cancelled",
            },
        )
    } else {
        save_model_download_notification(
            connection,
            ModelDownloadNotificationInput {
                actions: &[model_download_retry_action(catalog.id)],
                message: &format!(
                    "{} has no active or queued download to cancel.",
                    catalog.label
                ),
                model_id: catalog.id,
                preferred_notification_id,
                status: "warning",
                timestamp_ms,
                title: "Model download not running",
            },
        )
    }
}
