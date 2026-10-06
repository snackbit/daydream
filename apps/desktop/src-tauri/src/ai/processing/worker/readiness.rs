use super::super::super::*;

pub(in crate::ai) fn check_transcription_readiness<R: tauri::Runtime>(
    app: &AppHandle<R>,
    storage: &AppStorage,
    app_settings: &settings::AppSettings,
) -> Result<(), AiError> {
    match app_settings.ai.transcription.provider.as_str() {
        "local-whisper-cpp" => {
            let model_id = &app_settings.ai.transcription.local_model_id;
            let model = storage
                .with_connection(|connection| storage::get_ai_model_asset(connection, model_id))
                .map_err(storage_error)?
                .ok_or_else(|| {
                    ai_error(
                        "ai_processing_local_model_unavailable",
                        "Selected local transcription model has not been downloaded.",
                    )
                })?;
            let Some(local_path) = model.local_path.as_deref() else {
                return Err(ai_error(
                    "ai_processing_local_model_unavailable",
                    "Selected local transcription model has no local file.",
                ));
            };
            if model.download_status != "available" || !Path::new(local_path).is_file() {
                return Err(ai_error(
                    "ai_processing_local_model_unavailable",
                    "Selected local transcription model file is not available.",
                ));
            }
            let _ = app.path().app_data_dir().map_err(|error| {
                ai_error(
                    "ai_processing_app_data_unavailable",
                    &format!("App data directory is unavailable: {error}"),
                )
            })?;
            Ok(())
        }
        "openai-compatible" => {
            if app_settings
                .ai
                .transcription
                .remote_base_url
                .as_deref()
                .is_none_or(str::is_empty)
            {
                return Err(ai_error(
                    "ai_processing_remote_base_url_required",
                    "Remote transcription base URL is required.",
                ));
            }
            if app_settings
                .ai
                .transcription
                .remote_api_key_ref
                .as_deref()
                .is_none_or(str::is_empty)
            {
                return Err(ai_error(
                    "ai_processing_remote_api_key_required",
                    "Remote transcription API key is required.",
                ));
            }
            if app_settings
                .ai
                .transcription
                .remote_model
                .as_deref()
                .is_none_or(str::is_empty)
            {
                return Err(ai_error(
                    "ai_processing_remote_model_required",
                    "Remote transcription model is required.",
                ));
            }
            Ok(())
        }
        _ => Err(ai_error(
            "ai_processing_transcription_provider_invalid",
            "Transcription provider is not supported.",
        )),
    }
}

pub(in crate::ai) fn selected_transcription_model(
    app_settings: &settings::AppSettings,
) -> Result<String, AiError> {
    match app_settings.ai.transcription.provider.as_str() {
        "local-whisper-cpp" => Ok(app_settings.ai.transcription.local_model_id.clone()),
        "openai-compatible" => app_settings
            .ai
            .transcription
            .remote_model
            .clone()
            .filter(|model| !model.trim().is_empty())
            .ok_or_else(|| {
                ai_error(
                    "ai_processing_remote_model_required",
                    "Remote transcription model is required.",
                )
            }),
        _ => Err(ai_error(
            "ai_processing_transcription_provider_invalid",
            "Transcription provider is not supported.",
        )),
    }
}

pub(super) fn save_readiness_failure(
    storage: &AppStorage,
    recording_id: &str,
    code: &str,
    message: &str,
) -> AiError {
    let notification_message = format!("{message} Recording: {recording_id}.");
    let _ = storage.with_connection(|connection| {
        save_processing_notification(
            connection,
            ProcessingNotificationInput {
                actions: &[],
                job_id: None,
                message: &notification_message,
                recording_id,
                status: "warning",
                timestamp_ms: now_ms(),
                title: "AI processing unavailable",
            },
        )
        .map(|_row| ())
    });
    ai_error(code, message)
}
