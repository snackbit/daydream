use super::super::*;

#[tauri::command(rename = "listAiModels")]
pub fn list_ai_models(storage: State<'_, AppStorage>) -> Result<AiModelAssetList, AiError> {
    model_list(&storage)
}

#[tauri::command(rename = "downloadAiModel")]
pub fn download_ai_model(
    app: AppHandle,
    input: DownloadAiModelInput,
    manager: State<'_, AiModelManager>,
    storage: State<'_, AppStorage>,
) -> Result<Notification, AiError> {
    let catalog = ensure_catalog_model(&input.model_id)?;
    let existing_model_path = model_path(&app, catalog)?;
    if existing_model_path.is_file() && is_verified_model_file(&existing_model_path, catalog)? {
        let notification = storage
            .with_connection(|connection| {
                let timestamp_ms = now_ms();
                save_catalog_model(
                    connection,
                    catalog,
                    ModelAssetPatch {
                        download_status: "available".into(),
                        downloaded_bytes: catalog.size_bytes,
                        error_code: None,
                        local_path: Some(existing_model_path.to_string_lossy().into_owned().into()),
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
                        message: &format!("{} is already available locally.", catalog.label),
                        model_id: catalog.id,
                        preferred_notification_id: input.notification_id.as_deref(),
                        status: "succeeded",
                        timestamp_ms,
                        title: "Model already downloaded",
                    },
                )
            })
            .map_err(storage_error)?;
        emit_model_list(&app);

        return notification_from_row(notification);
    }
    if existing_model_path.is_file() {
        delete_file_if_exists(&existing_model_path)?;
    }

    let state = manager.request_download(catalog.id);
    let notification = storage
        .with_connection(|connection| {
            let timestamp_ms = now_ms();
            let status = if state == DownloadRequestState::Queued {
                "queued"
            } else {
                "downloading"
            };
            let patch = if state == DownloadRequestState::AlreadyRunning {
                ModelAssetPatch {
                    download_status: "downloading".into(),
                    ..ModelAssetPatch::from_existing(connection, catalog)?
                }
            } else {
                ModelAssetPatch {
                    download_status: status.into(),
                    downloaded_bytes: 0,
                    error_code: None,
                    etag: None,
                    local_path: None,
                    self_test_status: "not-run".into(),
                    total_bytes: Some(catalog.size_bytes),
                    verification_status: "pending".into(),
                }
            };
            save_catalog_model(connection, catalog, patch, timestamp_ms)?;
            save_model_download_notification(
                connection,
                ModelDownloadNotificationInput {
                    actions: &[model_download_cancel_action(catalog.id)],
                    message: &format!("{} will download from Hugging Face.", catalog.label),
                    model_id: catalog.id,
                    preferred_notification_id: input.notification_id.as_deref(),
                    status: status_to_notification_status(status),
                    timestamp_ms,
                    title: if state == DownloadRequestState::Queued {
                        "Model download queued"
                    } else if state == DownloadRequestState::AlreadyRunning {
                        "Model download already running"
                    } else {
                        "Model download started"
                    },
                },
            )
        })
        .map_err(storage_error)?;
    emit_model_list(&app);

    if state == DownloadRequestState::Started {
        spawn_model_download(app, catalog.id.to_string());
    }

    notification_from_row(notification)
}

#[tauri::command(rename = "cancelAiModelDownload")]
pub fn cancel_ai_model_download(
    app: AppHandle,
    input: CancelAiModelDownloadInput,
    manager: State<'_, AiModelManager>,
    storage: State<'_, AppStorage>,
) -> Result<Notification, AiError> {
    let catalog = ensure_catalog_model(&input.model_id)?;
    let active_or_queued_cancelled = manager.cancel_download(catalog.id);
    let persisted_cancelled = if active_or_queued_cancelled {
        false
    } else {
        storage
            .with_connection(|connection| storage::get_ai_model_asset(connection, catalog.id))
            .map_err(storage_error)?
            .is_some_and(|model| matches!(model.download_status.as_str(), "downloading" | "queued"))
    };
    let cancelled = active_or_queued_cancelled || persisted_cancelled;
    let downloaded = if cancelled {
        let partial_path = partial_model_path(&app, catalog)?;
        let final_path = model_path(&app, catalog)?;
        Some(downloaded_bytes(&partial_path, &final_path))
    } else {
        None
    };
    let notification = storage
        .with_connection(|connection| {
            let timestamp_ms = now_ms();
            save_cancel_download_result(
                connection,
                catalog,
                input.notification_id.as_deref(),
                cancelled,
                downloaded,
                timestamp_ms,
            )
        })
        .map_err(storage_error)?;
    if cancelled {
        emit_model_list(&app);
    }

    notification_from_row(notification)
}

#[tauri::command(rename = "deleteAiModel")]
pub fn delete_ai_model(
    app: AppHandle,
    input: DeleteAiModelInput,
    manager: State<'_, AiModelManager>,
    storage: State<'_, AppStorage>,
) -> Result<AiModelAssetList, AiError> {
    let catalog = ensure_catalog_model(&input.model_id)?;
    if manager.is_active_or_queued(catalog.id) {
        return Err(AiError {
            code: "ai_model_download_busy".to_string(),
            message: "Cancel the model download before deleting it.".to_string(),
        });
    }

    let path = model_path(&app, catalog)?;
    let partial_path = partial_model_path(&app, catalog)?;
    delete_file_if_exists(&path)?;
    delete_file_if_exists(&partial_path)?;

    storage
        .with_connection(|connection| {
            save_catalog_model(connection, catalog, ModelAssetPatch::missing(), now_ms())?;
            storage::list_ai_model_assets(connection)
        })
        .map_err(storage_error)
        .map(models_from_stored)
        .inspect(|_models| emit_model_list(&app))
}

#[tauri::command(rename = "selfTestAiModel")]
pub async fn self_test_ai_model(
    app: AppHandle,
    input: SelfTestAiModelInput,
    provider: State<'_, TranscriptionProviderManager>,
    storage: State<'_, AppStorage>,
) -> Result<Notification, AiError> {
    let catalog = ensure_catalog_model(&input.model_id)?;
    let path = model_path(&app, catalog)?;
    if !path.is_file() {
        return Err(AiError {
            code: "ai_model_missing".to_string(),
            message: "Download the model before running its load test.".to_string(),
        });
    }

    storage
        .with_connection(|connection| {
            save_catalog_model(
                connection,
                catalog,
                ModelAssetPatch {
                    self_test_status: "running".into(),
                    ..ModelAssetPatch::from_existing(connection, catalog)?
                },
                now_ms(),
            )
            .map(|_row| ())
        })
        .map_err(storage_error)?;
    emit_model_list(&app);

    let app_settings = settings::load_app_settings(&storage).map_err(settings_error)?;
    let warmup = LocalTranscriptionWarmup {
        local_backend: app_settings.ai.transcription.local_backend,
        model_id: catalog.id.to_string(),
        model_path: path.clone(),
    };
    let provider = provider.inner().clone();
    let load_result = tauri::async_runtime::spawn_blocking(move || {
        provider
            .warm_local_model(warmup)
            .map_err(|error| whisper_backend::WhisperBackendError {
                code: error.code,
                message: error.message,
            })
    })
    .await
    .map_err(|error| AiError {
        code: "ai_model_self_test_failed".to_string(),
        message: format!("Model load test task failed: {error}"),
    })?;

    let notification = storage
        .with_connection(|connection| {
            let timestamp_ms = now_ms();
            match load_result {
                Ok(()) => {
                    save_catalog_model(
                        connection,
                        catalog,
                        ModelAssetPatch {
                            self_test_status: "passed".into(),
                            ..ModelAssetPatch::from_existing(connection, catalog)?
                        },
                        timestamp_ms,
                    )?;
                    save_model_notification(
                        connection,
                        "succeeded",
                        "model-self-test",
                        "Model load test passed",
                        &format!("{} loaded successfully on CPU.", catalog.label),
                        &[],
                        timestamp_ms,
                    )
                }
                Err(error) => {
                    save_catalog_model(
                        connection,
                        catalog,
                        ModelAssetPatch {
                            error_code: Some(error.code.as_str().into()),
                            self_test_status: "failed".into(),
                            ..ModelAssetPatch::from_existing(connection, catalog)?
                        },
                        timestamp_ms,
                    )?;
                    save_model_notification(
                        connection,
                        "failed",
                        "model-self-test",
                        "Model load test failed",
                        &format!("{} could not be loaded: {}.", catalog.label, error.code),
                        &[],
                        timestamp_ms,
                    )
                }
            }
        })
        .map_err(storage_error)?;
    emit_model_list(&app);

    notification_from_row(notification)
}
