use std::{env, path::Path};

use tauri::{Manager, State};

use super::ai_processing::collect_ai_processing_diagnostics;
use super::latest_recording::collect_latest_recording_diagnostics;
use super::media_writer::collect_media_writer_diagnostics;
use super::ocr::collect_ocr_diagnostics;
use super::portal::collect_portal_restore_token_diagnostics;
use super::types::{
    AppDiagnostics, DaydreamDiagnostics, DebugLogBundle, StorageDiagnostics, SystemDiagnostics,
};
use crate::recorder::{self, RecorderManager};
use crate::storage::{collect_database_diagnostics, AppStorage};
use crate::telemetry;

#[tauri::command]
pub async fn get_diagnostics(
    app: tauri::AppHandle,
    storage: State<'_, AppStorage>,
    manager: State<'_, RecorderManager>,
) -> Result<DaydreamDiagnostics, String> {
    collect_diagnostics(app, &storage, &manager).await
}

#[tauri::command]
pub async fn get_debug_log_bundle(
    app: tauri::AppHandle,
    storage: State<'_, AppStorage>,
    manager: State<'_, RecorderManager>,
) -> Result<DebugLogBundle, String> {
    Ok(DebugLogBundle {
        diagnostics: collect_diagnostics(app, &storage, &manager).await?,
        log_tail_lines: telemetry::read_debug_log_tail()?,
    })
}

async fn collect_diagnostics(
    app: tauri::AppHandle,
    storage: &AppStorage,
    manager: &RecorderManager,
) -> Result<DaydreamDiagnostics, String> {
    let latest_recording = collect_latest_recording_diagnostics(storage, manager);
    Ok(DaydreamDiagnostics {
        app: AppDiagnostics {
            identifier: app.config().identifier.to_string(),
            name: "Daydream".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
        ai_processing: collect_ai_processing_diagnostics(
            storage,
            latest_recording.recording_id.as_deref(),
        ),
        database: collect_database_diagnostics(storage),
        debug_logging: telemetry::diagnostics(),
        latest_recording,
        media_writer: collect_media_writer_diagnostics(),
        ocr: collect_ocr_diagnostics(&app, storage),
        portal_restore_token: collect_portal_restore_token_diagnostics(storage),
        recorder: recorder::collect_backend_health().await,
        storage: StorageDiagnostics {
            app_data_dir: app
                .path()
                .app_data_dir()
                .ok()
                .map(|path| path_to_string(&path)),
        },
        system: SystemDiagnostics {
            arch: env::consts::ARCH.to_string(),
            family: env::consts::FAMILY.to_string(),
            platform: env::consts::OS.to_string(),
        },
    })
}

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().to_string()
}
