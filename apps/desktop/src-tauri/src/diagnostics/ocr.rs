use super::types::OcrDiagnostics;
use crate::{ai::resolve_tesseract_runtime, settings, storage};

pub(super) fn collect_ocr_diagnostics(
    app: &tauri::AppHandle,
    storage: &storage::AppStorage,
) -> OcrDiagnostics {
    let settings = settings::load_app_settings(storage).ok();
    let runtime = storage
        .with_connection(|connection| storage::get_ai_model_asset(connection, "tesseract-runtime"))
        .ok()
        .flatten();
    let data = ["tessdata-eng", "tessdata-deu"]
        .into_iter()
        .filter_map(|id| {
            storage
                .with_connection(|connection| storage::get_ai_model_asset(connection, id))
                .ok()
                .flatten()
        })
        .collect::<Vec<_>>();
    let resolved = resolve_tesseract_runtime(app).ok();

    OcrDiagnostics {
        data_download_status: resolved
            .as_ref()
            .map(|runtime| {
                if runtime.data_source == "partial" {
                    "partial".to_string()
                } else {
                    "available".to_string()
                }
            })
            .unwrap_or_else(|| data_status(&data)),
        data_path_present: resolved
            .as_ref()
            .and_then(|runtime| runtime.tessdata_dir.as_ref())
            .is_some_and(|path| path.is_dir())
            || data.iter().any(|asset| {
                asset
                    .local_path
                    .as_deref()
                    .is_some_and(|path| std::path::Path::new(path).is_file())
            }),
        data_source: resolved
            .as_ref()
            .map(|runtime| runtime.data_source.clone())
            .unwrap_or_else(|| "missing".to_string()),
        enabled: settings
            .as_ref()
            .map(|settings| settings.ai.ocr.enabled)
            .unwrap_or(false),
        engine: settings
            .map(|settings| settings.ai.ocr.engine)
            .unwrap_or_else(|| "unknown".to_string()),
        languages: resolved
            .as_ref()
            .map(|runtime| {
                runtime
                    .languages
                    .split('+')
                    .filter(|language| !language.is_empty())
                    .map(ToOwned::to_owned)
                    .collect()
            })
            .unwrap_or_default(),
        runtime_download_status: resolved
            .as_ref()
            .map(|_runtime| "available".to_string())
            .or_else(|| runtime.as_ref().map(|asset| asset.download_status.clone()))
            .unwrap_or_else(|| "missing".to_string()),
        runtime_path: resolved
            .as_ref()
            .and_then(|runtime| runtime.runtime_path.as_ref())
            .map(|path| path.to_string_lossy().to_string())
            .or_else(|| runtime.as_ref().and_then(|asset| asset.local_path.clone())),
        runtime_path_present: resolved
            .as_ref()
            .and_then(|runtime| runtime.runtime_path.as_ref())
            .is_some_and(|path| path.is_file())
            || runtime
                .as_ref()
                .and_then(|asset| asset.local_path.as_deref())
                .is_some_and(|path| std::path::Path::new(path).is_file()),
        runtime_source: resolved
            .as_ref()
            .map(|runtime| runtime.runtime_source.clone())
            .unwrap_or_else(|| "missing".to_string()),
        runtime_version: resolved
            .as_ref()
            .and_then(|runtime| runtime.runtime_version.clone()),
    }
}

fn data_status(data: &[storage::AiModelAssetRow]) -> String {
    if data.is_empty() {
        return "missing".to_string();
    }
    if data
        .iter()
        .all(|asset| asset.download_status == "available")
    {
        return "available".to_string();
    }
    if data
        .iter()
        .any(|asset| asset.download_status == "downloading")
    {
        return "downloading".to_string();
    }
    if data.iter().any(|asset| asset.download_status == "queued") {
        return "queued".to_string();
    }
    if data.iter().any(|asset| asset.download_status == "failed") {
        return "failed".to_string();
    }
    if data
        .iter()
        .any(|asset| asset.download_status == "available")
    {
        return "partial".to_string();
    }

    "missing".to_string()
}
