use super::super::*;
use super::{emit_model_list, save_catalog_model, CatalogModel, ModelAssetPatch};

pub(in crate::ai) fn model_transfer_action(
    model_id: &str,
    paused: bool,
) -> crate::notifications::NotificationAction {
    let mut action = model_download_cancel_action(model_id);
    action.id = if paused { "resume" } else { "pause" }.into();
    action.handler = if paused {
        "model-download-resume"
    } else {
        "model-download-pause"
    }
    .into();
    action.label = if paused { "Resume" } else { "Pause" }.into();
    action.style = "secondary".into();
    action
}

pub async fn cancel_transfers_for_maintenance(app: &AppHandle) -> Result<(), String> {
    app.state::<AiModelManager>()
        .cancel_and_wait_for_maintenance()
        .await;
    app.state::<AppStorage>().with_connection(|connection| {
        let transaction=connection.unchecked_transaction()?;
        transaction.execute("UPDATE ai_model_assets SET download_status='cancelled',error_code='ai_model_download_cancelled',updated_at_ms=?1 WHERE download_status IN ('queued','downloading')",[now_ms()])?;
        transaction.execute("UPDATE ai_model_download_runtime SET control='cancel',phase='cancelled' WHERE model_id IN (SELECT id FROM ai_model_assets WHERE download_status='cancelled')",[])?;
        transaction.commit()
    }).map_err(|_|"Could not save stopped model downloads. Retry maintenance.".to_string())?;
    emit_model_list(app);
    Ok(())
}

pub(in crate::ai) fn save_stopped_download(
    app: &AppHandle,
    catalog: &CatalogModel,
    downloaded: i64,
) -> Result<(), AiError> {
    let manager = app.state::<AiModelManager>();
    if !manager.is_paused(catalog.id) {
        manager.set_telemetry(catalog.id, "cancelled", None, None);
        return save_cancelled_download(app, catalog, downloaded);
    }
    app.state::<AppStorage>().with_connection(|connection| {
        save_catalog_model(connection,catalog,ModelAssetPatch { download_status: "paused".into(),downloaded_bytes:downloaded,error_code:None,..ModelAssetPatch::from_existing(connection,catalog)? },now_ms())?;
        connection.execute("UPDATE ai_model_download_runtime SET control='pause',phase='paused' WHERE model_id=?1",[catalog.id])?;
        save_model_download_notification(connection,ModelDownloadNotificationInput {actions:&[model_transfer_action(catalog.id,true),model_download_cancel_action(catalog.id)],message:&format!("{} is paused. Its partial data is saved for resume.",catalog.label),model_id:catalog.id,preferred_notification_id:None,status:"info",timestamp_ms:now_ms(),title:"Model download paused"})?;
        Ok(())
    }).map_err(storage_error)?;
    manager.set_telemetry(catalog.id, "paused", None, None);
    emit_model_list(app);
    Ok(())
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
