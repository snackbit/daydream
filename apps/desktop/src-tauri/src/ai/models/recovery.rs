use super::super::*;
use super::{
    catalog_model, delete_file_if_exists, emit_model_list, is_verified_model_file, model_path,
    partial_model_path, prepare_partial_download, save_catalog_model, spawn_model_download,
    CatalogModel, DownloadRequestState, ModelAssetPatch, PartialDownloadResume,
};

pub fn recover_model_downloads(app: AppHandle) {
    if let Err(error) = recover_model_downloads_inner(&app) {
        tracing::warn!(
            error_code = %error.code,
            "AI model download recovery failed."
        );
    }
}

pub(in crate::ai) fn recover_model_downloads_inner(app: &AppHandle) -> Result<(), AiError> {
    let storage = app.state::<AppStorage>();
    let manager = app.state::<AiModelManager>();
    let mut recoverable = storage
        .with_connection(storage::list_ai_model_assets)
        .map_err(storage_error)?
        .into_iter()
        .filter_map(|row| {
            if !matches!(row.download_status.as_str(), "downloading" | "queued") {
                return None;
            }
            catalog_model(&row.id).map(|catalog| (catalog, row))
        })
        .collect::<Vec<_>>();

    recoverable
        .sort_by_key(|(_catalog, row)| (row.download_status != "downloading", row.updated_at_ms));

    for (catalog, row) in recoverable {
        recover_model_download(app, &storage, &manager, catalog, &row)?;
    }

    Ok(())
}

pub(in crate::ai) fn recover_model_download(
    app: &AppHandle,
    storage: &AppStorage,
    manager: &AiModelManager,
    catalog: &'static CatalogModel,
    row: &AiModelAssetRow,
) -> Result<(), AiError> {
    let final_path = model_path(app, catalog)?;
    let partial_path = partial_model_path(app, catalog)?;
    match recover_model_files(catalog, &partial_path, &final_path)? {
        RecoveredModelFiles::VerifiedFinal => {
            let local_path = final_path.to_string_lossy().to_string();
            storage
                .with_connection(|connection| {
                    let timestamp_ms = now_ms();
                    save_catalog_model(
                        connection,
                        catalog,
                        ModelAssetPatch {
                            download_status: "available".into(),
                            downloaded_bytes: catalog.size_bytes,
                            error_code: None,
                            local_path: Some(local_path.as_str().into()),
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
                            message: &format!(
                                "{} was recovered from a verified local file.",
                                catalog.label
                            ),
                            model_id: catalog.id,
                            preferred_notification_id: None,
                            status: "succeeded",
                            timestamp_ms,
                            title: "Model download recovered",
                        },
                    )
                    .map(|_row| ())
                })
                .map_err(storage_error)?;
            emit_model_list(app);
        }
        RecoveredModelFiles::Partial(downloaded_bytes) => {
            resume_recovered_model_download(app, storage, manager, catalog, downloaded_bytes)?;
        }
        RecoveredModelFiles::None => {
            if row.download_status == "queued" && row.downloaded_bytes == 0 {
                resume_recovered_model_download(app, storage, manager, catalog, 0)?;
            } else {
                storage
                    .with_connection(|connection| {
                        let timestamp_ms = now_ms();
                        save_catalog_model(
                            connection,
                            catalog,
                            ModelAssetPatch {
                                download_status: "failed".into(),
                                downloaded_bytes: 0,
                                error_code: Some("ai_model_download_partial_missing".into()),
                                local_path: None,
                                verification_status: "failed".into(),
                                ..ModelAssetPatch::from_existing(connection, catalog)?
                            },
                            timestamp_ms,
                        )?;
                        save_model_download_notification(
                            connection,
                            ModelDownloadNotificationInput {
                                actions: &[model_download_retry_action(catalog.id)],
                                message: &format!(
                                    "{} download could not resume because its partial file is missing.",
                                    catalog.label
                                ),
                                model_id: catalog.id,
                                preferred_notification_id: None,
                                status: "failed",
                                timestamp_ms,
                                title: "Model download needs retry",
                            },
                        )
                        .map(|_row| ())
                    })
                    .map_err(storage_error)?;
                emit_model_list(app);
            }
        }
    }

    Ok(())
}

fn resume_recovered_model_download(
    app: &AppHandle,
    storage: &AppStorage,
    manager: &AiModelManager,
    catalog: &'static CatalogModel,
    downloaded_bytes: i64,
) -> Result<(), AiError> {
    let state = manager.request_download(catalog.id);
    let status = if state == DownloadRequestState::Queued {
        "queued"
    } else {
        "downloading"
    };
    storage
        .with_connection(|connection| {
            let timestamp_ms = now_ms();
            save_catalog_model(
                connection,
                catalog,
                ModelAssetPatch {
                    download_status: status.into(),
                    downloaded_bytes,
                    error_code: None,
                    local_path: None,
                    total_bytes: Some(catalog.size_bytes),
                    verification_status: "pending".into(),
                    ..ModelAssetPatch::from_existing(connection, catalog)?
                },
                timestamp_ms,
            )?;
            save_model_download_notification(
                connection,
                ModelDownloadNotificationInput {
                    actions: &[model_download_cancel_action(catalog.id)],
                    message: &format!(
                        "{} download will resume from local partial data.",
                        catalog.label
                    ),
                    model_id: catalog.id,
                    preferred_notification_id: None,
                    status: status_to_notification_status(status),
                    timestamp_ms,
                    title: if state == DownloadRequestState::Queued {
                        "Model download queued"
                    } else {
                        "Model download resumed"
                    },
                },
            )
            .map(|_row| ())
        })
        .map_err(storage_error)?;
    emit_model_list(app);

    if state == DownloadRequestState::Started {
        spawn_model_download(app.clone(), catalog.id.to_string());
    }

    Ok(())
}

#[derive(Debug, PartialEq, Eq)]
pub(in crate::ai) enum RecoveredModelFiles {
    None,
    Partial(i64),
    VerifiedFinal,
}

pub(in crate::ai) fn recover_model_files(
    catalog: &CatalogModel,
    partial_path: &Path,
    final_path: &Path,
) -> Result<RecoveredModelFiles, AiError> {
    if final_path.is_file() {
        if is_verified_model_file(final_path, catalog)? {
            return Ok(RecoveredModelFiles::VerifiedFinal);
        }
        delete_file_if_exists(final_path)?;
    }

    match prepare_partial_download(catalog, partial_path)? {
        PartialDownloadResume::Complete => Ok(RecoveredModelFiles::Partial(catalog.size_bytes)),
        PartialDownloadResume::Resume(length) if length > 0 => {
            Ok(RecoveredModelFiles::Partial(length as i64))
        }
        PartialDownloadResume::Resume(_length) => Ok(RecoveredModelFiles::None),
    }
}
