use super::super::*;
use super::finalize::{finalize_complete_partial_if_valid, finalize_model_download};
use super::transfer_state::{save_download_progress, save_stopped_download};
use super::transport::{read_chunk, send_request, validate_content_range, StreamRead};
use super::{
    catalog_model, checksum_file, delete_file_if_exists, emit_model_list, ensure_catalog_model,
    model_path, partial_model_path, require_model_source_url, save_catalog_model, CatalogModel,
    ModelAssetPatch,
};

pub(in crate::ai) fn spawn_model_download(app: AppHandle, model_id: String) {
    let lease = match app
        .state::<crate::maintenance::runtime::RuntimeActivityRegistry>()
        .begin("model-download")
    {
        Ok(lease) => lease,
        Err(_) => {
            app.state::<AiModelManager>()
                .abort_worker_admission(&model_id);
            return;
        }
    };
    tauri::async_runtime::spawn(async move {
        let _lease = lease;
        let result = run_model_download(&app, &model_id).await;
        if let Err(error) = result {
            app.state::<AiModelManager>()
                .set_telemetry(&model_id, "failed", None, None);
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
                            message: &format!(
                                "{} could not be downloaded. Retry to resume its saved data.",
                                catalog.label
                            ),
                            model_id: catalog.id,
                            preferred_notification_id: None,
                            status: "failed",
                            timestamp_ms,
                            title: "Model download failed",
                        },
                    )
                    .map(|_row| ())
                });
            }
        }

        let manager = app.state::<AiModelManager>();
        if let Some(next_model_id) = manager.finish_download(&model_id) {
            spawn_model_download(app.clone(), next_model_id);
        }
        emit_model_list(&app);
    });
}

pub(in crate::ai) async fn run_model_download(
    app: &AppHandle,
    model_id: &str,
) -> Result<(), AiError> {
    let catalog = ensure_catalog_model(model_id)?;
    let storage = app.state::<AppStorage>();
    let manager = app.state::<AiModelManager>();
    let attempt = storage
        .with_connection(|connection| {
            connection.query_row(
                "SELECT attempt FROM ai_model_download_runtime WHERE model_id=?1",
                [model_id],
                |row| row.get::<_, i64>(0),
            )
        })
        .map_err(storage_error)?;
    manager.begin_attempt(model_id, attempt);
    emit_model_list(app);
    let final_path = model_path(app, catalog)?;
    let partial_path = partial_model_path(app, catalog)?;
    let source_url = require_model_source_url(catalog)?;
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
        let mut request = client.get(source_url.clone());
        if partial_len > 0 {
            request = request.header(RANGE, format!("bytes={partial_len}-"));
            if let Some(etag) = existing.as_ref().and_then(|row| row.etag.clone()) {
                request = request.header(IF_RANGE, etag);
            }
        }
        let Some(response) = send_request(request, &manager, catalog.id).await? else {
            save_stopped_download(app, catalog, partial_len as i64)?;
            return Ok(());
        };
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
        let restarted = partial_len > 0 && status == StatusCode::OK;
        if status == StatusCode::PARTIAL_CONTENT {
            validate_content_range(
                response
                    .headers()
                    .get(reqwest::header::CONTENT_RANGE)
                    .and_then(|value| value.to_str().ok()),
                partial_len,
                catalog.size_bytes,
            )?;
        }
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
        let total_bytes = response_len
            .map(|length| length + partial_len as i64)
            .or(Some(catalog.size_bytes));
        if total_bytes.is_some_and(|total| total != catalog.size_bytes) {
            return Err(AiError {
                code: "ai_model_download_size_mismatch".into(),
                message: "The server returned an unexpected file size. Retry the download.".into(),
            });
        }
        manager.set_telemetry(
            catalog.id,
            if restarted {
                "restarting"
            } else {
                "transferring"
            },
            None,
            None,
        );

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
        loop {
            let chunk = match read_chunk(&mut stream, &manager, catalog.id).await {
                StreamRead::Chunk(chunk) => chunk,
                StreamRead::Stopped => {
                    file.flush().map_err(file_error)?;
                    file.sync_data().map_err(file_error)?;
                    drop(file);
                    save_stopped_download(app, catalog, downloaded)?;
                    return Ok(());
                }
                StreamRead::Stalled => {
                    manager.set_telemetry(catalog.id, "stalled", None, None);
                    emit_model_list(app);
                    continue;
                }
            };
            let Some(chunk) = chunk else {
                break;
            };
            let chunk = chunk.map_err(download_error)?;
            file.write_all(&chunk).map_err(file_error)?;
            downloaded += chunk.len() as i64;
            if downloaded - last_progress_bytes >= MODEL_PROGRESS_UPDATE_BYTES
                || last_progress_update.elapsed() >= MODEL_PROGRESS_UPDATE_INTERVAL
            {
                let elapsed = last_progress_update.elapsed().as_secs_f64();
                let speed = ((downloaded - last_progress_bytes) as f64 / elapsed.max(0.001)) as i64;
                let eta = total_bytes
                    .filter(|_| speed > 0)
                    .map(|total| (total - downloaded).max(0) * 1000 / speed);
                manager.set_telemetry(catalog.id, "transferring", Some(speed), eta);
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
        if checksum_file(partial_path, catalog)? == catalog.sha1 {
            return Ok(PartialDownloadResume::Complete);
        }
        delete_file_if_exists(partial_path)?;
        return Ok(PartialDownloadResume::Resume(0));
    }

    Ok(PartialDownloadResume::Resume(partial_len))
}
