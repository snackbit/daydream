use super::super::*;

#[tauri::command(rename = "getAiProviderStatus")]
pub fn get_ai_provider_status(storage: State<'_, AppStorage>) -> Result<AiProviderStatus, AiError> {
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
        } else if app_settings.ai.vision.api_key_ref.is_some() {
            "configured".to_string()
        } else {
            "missing-key".to_string()
        },
    })
}

#[tauri::command(rename = "saveAiProviderSecret")]
pub fn save_ai_provider_secret(
    input: SaveAiProviderSecretInput,
    storage: State<'_, AppStorage>,
) -> Result<SaveAiProviderSecretResult, AiError> {
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
    storage: State<'_, AppStorage>,
) -> Result<(), AiError> {
    if !input.secret_ref.starts_with("ai.secret.") {
        return Err(AiError {
            code: "ai_secret_ref_invalid".to_string(),
            message: "AI secret reference is invalid.".to_string(),
        });
    }

    storage
        .with_connection(|connection| storage::delete_app_setting(connection, &input.secret_ref))
        .map_err(storage_error)
}
