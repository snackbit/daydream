use super::super::*;
use super::model_commands::admit_model_action;

#[tauri::command(rename = "getAiProviderStatus")]
pub fn get_ai_provider_status(
    app: AppHandle,
    storage: State<'_, AppStorage>,
) -> Result<AiProviderStatus, AiError> {
    let app_settings = settings::load_app_settings(&storage).map_err(|error| AiError {
        code: error.code,
        message: error.message,
    })?;
    let models = model_list(&storage)?;
    let selected = models
        .models
        .iter()
        .find(|model| model.id == app_settings.ai.transcription.local_model_id)
        .or_else(|| models.models.iter().find(|model| model.id == "small"));
    let ocr_runtime = models
        .models
        .iter()
        .find(|model| model.id == "tesseract-runtime");
    let ocr_data = models
        .models
        .iter()
        .filter(|model| matches!(model.id.as_str(), "tessdata-eng" | "tessdata-deu"))
        .collect::<Vec<_>>();
    let ocr_runtime_status = ocr_runtime_status(&app, ocr_runtime, &ocr_data);
    let backend_status = whisper_backend::backend_status();

    Ok(AiProviderStatus {
        local_transcription: AiLocalTranscriptionStatus {
            available_languages: whisper_backend::supported_languages()
                .into_iter()
                .map(|language| AiTranscriptionLanguageStatus {
                    id: language.id,
                    label: language.label,
                })
                .collect(),
            available_backends: vec![
                AiBackendStatus {
                    available: true,
                    id: "cpu".to_string(),
                    reason: None,
                },
                AiBackendStatus {
                    available: backend_status.vulkan_compiled,
                    id: "vulkan".to_string(),
                    reason: (!backend_status.vulkan_compiled)
                        .then(|| "Vulkan backend is not compiled in this build.".to_string()),
                },
                AiBackendStatus {
                    available: backend_status.cuda_compiled,
                    id: "cuda".to_string(),
                    reason: (!backend_status.cuda_compiled)
                        .then(|| "CUDA backend is not compiled in this build.".to_string()),
                },
            ],
            checksum_status: selected
                .map(|model| model.verification_status.clone())
                .unwrap_or_else(|| "unknown".to_string()),
            local_model_id: app_settings.ai.transcription.local_model_id.clone(),
            model_download_status: selected
                .map(|model| model.download_status.clone())
                .unwrap_or_else(|| "missing".to_string()),
            model_path: selected.and_then(|model| model.local_path.clone()),
            selected_backend: app_settings.ai.transcription.local_backend.clone(),
            self_test_status: selected
                .map(|model| model.self_test_status.clone())
                .unwrap_or_else(|| "not-run".to_string()),
        },
        ocr: AiOcrStatus {
            data_download_status: ocr_runtime_status.data_download_status,
            data_path: ocr_runtime_status.data_path,
            data_source: ocr_runtime_status.data_source,
            enabled: app_settings.ai.ocr.enabled,
            engine: app_settings.ai.ocr.engine.clone(),
            languages: ocr_runtime_status.languages,
            runtime_download_status: ocr_runtime_status.runtime_download_status,
            runtime_path: ocr_runtime_status.runtime_path,
            runtime_source: ocr_runtime_status.runtime_source,
            runtime_version: ocr_runtime_status.runtime_version,
        },
        remote_transcription: if app_settings.ai.transcription.provider == "openai-compatible" {
            if app_settings.ai.transcription.remote_api_key_ref.is_some() {
                "configured".to_string()
            } else {
                "missing-key".to_string()
            }
        } else {
            "not-configured".to_string()
        },
        vision: if app_settings.ai.vision.provider == "disabled" {
            "disabled".to_string()
        } else if settings::vision_ready(&app_settings.ai.vision) {
            "configured".to_string()
        } else {
            "not-ready".to_string()
        },
    })
}

struct OcrRuntimeStatus {
    data_download_status: String,
    data_path: Option<String>,
    data_source: String,
    languages: Vec<String>,
    runtime_download_status: String,
    runtime_path: Option<String>,
    runtime_source: String,
    runtime_version: Option<String>,
}

fn ocr_runtime_status(
    app: &AppHandle,
    ocr_runtime: Option<&AiModelAsset>,
    ocr_data: &[&AiModelAsset],
) -> OcrRuntimeStatus {
    match resolve_tesseract_runtime(app) {
        Ok(resolved) => OcrRuntimeStatus {
            data_download_status: if resolved.data_source == "partial" {
                "partial".to_string()
            } else {
                "available".to_string()
            },
            data_path: resolved
                .tessdata_dir
                .as_ref()
                .map(|path| path.to_string_lossy().to_string()),
            data_source: resolved.data_source,
            languages: resolved
                .languages
                .split('+')
                .filter(|language| !language.is_empty())
                .map(ToOwned::to_owned)
                .collect(),
            runtime_download_status: "available".to_string(),
            runtime_path: resolved
                .runtime_path
                .as_ref()
                .map(|path| path.to_string_lossy().to_string()),
            runtime_source: resolved.runtime_source,
            runtime_version: resolved.runtime_version,
        },
        Err(_error) => OcrRuntimeStatus {
            data_download_status: ocr_data_status(ocr_data),
            data_path: ocr_data.iter().find_map(|model| model.local_path.clone()),
            data_source: "missing".to_string(),
            languages: Vec::new(),
            runtime_download_status: ocr_runtime
                .map(|model| model.download_status.clone())
                .unwrap_or_else(|| "missing".to_string()),
            runtime_path: ocr_runtime.and_then(|model| model.local_path.clone()),
            runtime_source: "missing".to_string(),
            runtime_version: None,
        },
    }
}

fn ocr_data_status(models: &[&AiModelAsset]) -> String {
    if models.is_empty() {
        return "missing".to_string();
    }
    if models
        .iter()
        .all(|model| model.download_status == "available")
    {
        return "available".to_string();
    }
    if models
        .iter()
        .any(|model| model.download_status == "downloading")
    {
        return "downloading".to_string();
    }
    if models.iter().any(|model| model.download_status == "queued") {
        return "queued".to_string();
    }
    if models.iter().any(|model| model.download_status == "failed") {
        return "failed".to_string();
    }
    if models
        .iter()
        .any(|model| model.download_status == "available")
    {
        return "partial".to_string();
    }

    "missing".to_string()
}

#[tauri::command(rename = "saveAiProviderSecret")]
pub fn save_ai_provider_secret(
    input: SaveAiProviderSecretInput,
    app: AppHandle,
    storage: State<'_, AppStorage>,
) -> Result<SaveAiProviderSecretResult, AiError> {
    let _activity = admit_model_action(&app, "model-provider-secret-save")?;
    if input.role != "transcription" && input.role != "vision" {
        return Err(AiError {
            code: "ai_secret_role_invalid".to_string(),
            message: "AI secret role must be transcription or vision.".to_string(),
        });
    }
    if input.plaintext.trim().is_empty() {
        return Err(AiError {
            code: "ai_secret_empty".to_string(),
            message: "API key cannot be empty.".to_string(),
        });
    }

    let envelope =
        secret_crypto::encrypt_sensitive_setting_with_platform_key(input.plaintext.trim())
            .map_err(secret_error)?;
    let secret_ref = format!("ai.secret.{}.{}", input.role, Uuid::now_v7());
    storage
        .with_connection(|connection| {
            storage::save_app_setting(
                connection,
                &SaveAppSettingInput {
                    key: secret_ref.clone(),
                    value_json: envelope,
                    is_sensitive: true,
                    timestamp_ms: now_ms(),
                },
            )?;
            Ok(())
        })
        .map_err(storage_error)?;

    Ok(SaveAiProviderSecretResult { secret_ref })
}

#[tauri::command(rename = "deleteAiProviderSecret")]
pub fn delete_ai_provider_secret(
    input: DeleteAiProviderSecretInput,
    app: AppHandle,
    storage: State<'_, AppStorage>,
) -> Result<(), AiError> {
    let _activity = admit_model_action(&app, "model-provider-secret-delete")?;
    if !input.secret_ref.starts_with("ai.secret.") {
        return Err(AiError {
            code: "ai_secret_ref_invalid".to_string(),
            message: "AI secret reference is invalid.".to_string(),
        });
    }

    storage
        .with_connection(|connection| {
            settings::prune_unreferenced_secrets(connection, Some(&input.secret_ref)).map(|_| ())
        })
        .map_err(storage_error)
}
