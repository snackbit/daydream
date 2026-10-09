use super::super::*;

#[path = "model_commands/admission.rs"]
mod admission;
pub(super) use admission::admit_model_action;
use admission::with_model_admission;

#[tauri::command(rename = "listAiModels")]
pub fn list_ai_models(
    storage: State<'_, AppStorage>,
    manager: State<'_, AiModelManager>,
) -> Result<AiModelAssetList, AiError> {
    let mut models = model_list(&storage)?;
    manager.decorate(&mut models);
    Ok(models)
}

#[tauri::command(rename = "downloadAiModel")]
pub fn download_ai_model(
    app: AppHandle,
    input: DownloadAiModelInput,
    manager: State<'_, AiModelManager>,
    storage: State<'_, AppStorage>,
) -> Result<Notification, AiError> {
    let action_app = app.clone();
    with_model_admission(&app, "model-download-command", || {
        download_model(action_app, input, manager, storage)
    })
}

fn download_model(
    app: AppHandle,
    input: DownloadAiModelInput,
    manager: State<'_, AiModelManager>,
    storage: State<'_, AppStorage>,
) -> Result<Notification, AiError> {
    let _command = manager.command_guard();
    let catalog = ensure_catalog_model(&input.model_id)?;
    require_model_source_url(catalog)?;
    let existing_model_path = model_path(&app, catalog)?;
    if existing_model_path.is_file() && is_verified_model_file(&existing_model_path, catalog)? {
        if manager.is_active_or_queued(catalog.id) {
            return Err(AiError {
                code: "ai_model_download_busy".into(),
                message: "The model is still being verified or tested. Wait for it to finish."
                    .into(),
            });
        }
        let notification = storage
            .with_connection(|connection| {
                let transaction = connection.unchecked_transaction()?;
                let connection = &transaction;
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
                let notification = save_model_download_notification(
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
                )?;
                transaction.commit()?;
                Ok(notification)
            })
            .map_err(storage_error)?;
        emit_model_list(&app);

        return notification_from_row(notification);
    }
    if existing_model_path.is_file() {
        delete_file_if_exists(&existing_model_path)?;
    }

    let (state, notification) = manager.admit(catalog.id, |state| storage
        .with_connection(|connection| {
            let transaction = connection.unchecked_transaction()?;
            let connection = &transaction;
            let timestamp_ms = now_ms();
            let previous=ModelAssetPatch::from_existing(connection,catalog)?;
            let duplicate = *state == DownloadRequestState::AlreadyRunning || (*state == DownloadRequestState::Queued && previous.download_status == "queued");
            let status = if *state == DownloadRequestState::Queued {
                "queued"
            } else {
                "downloading"
            };
            let patch = if *state == DownloadRequestState::AlreadyRunning {
                previous
            } else {
                ModelAssetPatch {
                    download_status: status.into(),
                    downloaded_bytes: std::fs::metadata(partial_model_path(&app, catalog).map_err(|_| rusqlite::Error::InvalidQuery)?).map(|metadata| metadata.len() as i64).unwrap_or(0),
                    error_code: None,
                    etag: ModelAssetPatch::from_existing(connection, catalog)?.etag,
                    local_path: None,
                    self_test_status: "not-run".into(),
                    total_bytes: Some(catalog.size_bytes),
                    verification_status: "pending".into(),
                }
            };
            save_catalog_model(connection, catalog, patch, timestamp_ms)?;
            let notification = save_model_download_notification(
                connection,
                ModelDownloadNotificationInput {
                    actions: &[model_transfer_action(catalog.id,false),model_download_cancel_action(catalog.id)],
                    message: &format!("{} download requested.", catalog.label),
                    model_id: catalog.id,
                    preferred_notification_id: input.notification_id.as_deref(),
                    status: status_to_notification_status(status),
                    timestamp_ms,
                    title: if *state == DownloadRequestState::Queued {
                        "Model download queued"
                    } else if *state == DownloadRequestState::AlreadyRunning {
                        "Model download already running"
                    } else {
                        "Model download started"
                    },
                },
            )?;
            if !duplicate { connection.execute("INSERT INTO ai_model_download_runtime(model_id,phase,queue_order) VALUES (?1,?2,?3) ON CONFLICT(model_id) DO UPDATE SET attempt=attempt+1, phase=excluded.phase, control='run',queue_order=excluded.queue_order", rusqlite::params![catalog.id, if status=="queued" {"queued"} else {"connecting"}, timestamp_ms])?; }
            transaction.commit()?;
            Ok(notification)
        })
        .map_err(storage_error))?;
    emit_model_list(&app);

    if state == DownloadRequestState::Started {
        spawn_model_download(app, catalog.id.to_string());
    }

    notification_from_row(notification)
}

#[tauri::command(rename = "pauseAiModelDownload")]
pub async fn pause_ai_model_download(
    app: AppHandle,
    input: CancelAiModelDownloadInput,
    manager: State<'_, AiModelManager>,
    storage: State<'_, AppStorage>,
) -> Result<AiModelAssetList, AiError> {
    let _activity = admit_model_action(&app, "model-pause-command")?;
    let catalog = ensure_catalog_model(&input.model_id)?;
    let status = storage
        .with_connection(|connection| storage::get_ai_model_asset(connection, catalog.id))
        .map_err(storage_error)?;
    if !status.is_some_and(|row| {
        matches!(
            row.download_status.as_str(),
            "downloading" | "queued" | "paused"
        )
    }) {
        return Err(AiError {
            code: "ai_model_download_not_running".into(),
            message: "This model has no download to pause.".into(),
        });
    }
    storage.with_connection(|connection| {
        connection.execute("UPDATE ai_model_download_runtime SET control='pause',phase='pausing' WHERE model_id=?1", [catalog.id])?;
        save_catalog_model(connection, catalog, ModelAssetPatch { download_status: "paused".into(), error_code: None, ..ModelAssetPatch::from_existing(connection,catalog)? }, now_ms()).map(|_| ())
    }).map_err(storage_error)?;
    manager.pause_download(catalog.id);
    manager.wait_for_model(catalog.id).await;
    save_stopped_download(
        &app,
        catalog,
        downloaded_bytes(
            &partial_model_path(&app, catalog)?,
            &model_path(&app, catalog)?,
        ),
    )?;
    model_list(&storage)
}

#[tauri::command(rename = "resumeAiModelDownload")]
pub fn resume_ai_model_download(
    app: AppHandle,
    input: DownloadAiModelInput,
    manager: State<'_, AiModelManager>,
    storage: State<'_, AppStorage>,
) -> Result<Notification, AiError> {
    let _activity = admit_model_action(&app, "model-resume-command")?;
    download_model(app, input, manager, storage)
}

#[tauri::command(rename = "removeAiModelPartial")]
pub fn remove_ai_model_partial(
    app: AppHandle,
    input: DeleteAiModelInput,
    manager: State<'_, AiModelManager>,
    storage: State<'_, AppStorage>,
) -> Result<AiModelAssetList, AiError> {
    let _activity = admit_model_action(&app, "model-remove-partial-command")?;
    let _command = manager.command_guard();
    let catalog = ensure_catalog_model(&input.model_id)?;
    if manager.is_active_or_queued(catalog.id) {
        return Err(AiError {
            code: "ai_model_download_busy".into(),
            message: "Pause the download before removing its partial data.".into(),
        });
    }
    delete_file_if_exists(&partial_model_path(&app, catalog)?)?;
    storage
        .with_connection(|connection| {
            save_catalog_model(connection, catalog, ModelAssetPatch::missing(), now_ms())?;
            connection.execute(
                "DELETE FROM ai_model_download_runtime WHERE model_id=?1",
                [catalog.id],
            )?;
            Ok(())
        })
        .map_err(storage_error)?;
    emit_model_list(&app);
    model_list(&storage)
}

#[tauri::command(rename = "cancelAiModelDownload")]
pub async fn cancel_ai_model_download(
    app: AppHandle,
    input: CancelAiModelDownloadInput,
    manager: State<'_, AiModelManager>,
    storage: State<'_, AppStorage>,
) -> Result<Notification, AiError> {
    let _activity = admit_model_action(&app, "model-cancel-command")?;
    let catalog = ensure_catalog_model(&input.model_id)?;
    let active_or_queued_cancelled = manager.cancel_download(catalog.id);
    let persisted_cancelled = if active_or_queued_cancelled {
        false
    } else {
        storage
            .with_connection(|connection| storage::get_ai_model_asset(connection, catalog.id))
            .map_err(storage_error)?
            .is_some_and(|model| {
                matches!(
                    model.download_status.as_str(),
                    "downloading" | "queued" | "paused"
                )
            })
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
            let result=save_cancel_download_result(
                connection,
                catalog,
                input.notification_id.as_deref(),
                cancelled,
                downloaded,
                timestamp_ms,
            )?;
            if cancelled { connection.execute("UPDATE ai_model_download_runtime SET control='cancel',phase='cancelled' WHERE model_id=?1",[catalog.id])?; }
            Ok(result)
        })
        .map_err(storage_error)?;
    if cancelled {
        manager.wait_for_model(catalog.id).await;
        manager.set_telemetry(catalog.id, "cancelled", None, None);
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
    let _activity = admit_model_action(&app, "model-delete-command")?;
    let _command = manager.command_guard();
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
    let activity = Arc::new(admit_model_action(&app, "model-self-test-command")?);
    let catalog = ensure_catalog_model(&input.model_id)?;
    let manager = app.state::<AiModelManager>();
    let _model_use = manager.use_model(catalog.id)?;
    if !is_whisper_model(catalog) {
        return Err(AiError {
            code: "ai_model_self_test_unsupported".to_string(),
            message: "Only local Whisper models support load tests.".to_string(),
        });
    }
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
    let worker_activity = activity.clone();
    let load_result = tauri::async_runtime::spawn_blocking(move || {
        let _activity = worker_activity;
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
