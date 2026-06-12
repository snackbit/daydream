use super::super::super::*;

pub(crate) fn enqueue_recording_ai_processing<R: tauri::Runtime>(
    app: &AppHandle<R>,
    storage: &AppStorage,
    recording_id: &str,
) -> Result<ProcessingJobRow, AiError> {
    tracing::info!(recording_id = %recording_id, "ai_processing_start_requested");
    let app_settings = settings::load_app_settings(storage).map_err(settings_error)?;
    let recording = storage
        .with_connection(|connection| storage::get_recording(connection, recording_id))
        .map_err(storage_error)?;
    if recording.status != "completed" {
        tracing::warn!(
            recording_id = %recording_id,
            recording_status = %recording.status,
            "ai_processing_start_blocked_recording_not_completed"
        );
        return Err(save_readiness_failure(
            storage,
            recording_id,
            "ai_processing_recording_not_completed",
            "Recording must be completed before AI processing starts.",
        ));
    }
    let segments = storage
        .with_connection(|connection| storage::list_recording_segments(connection, recording_id))
        .map_err(storage_error)?;
    if storage
        .with_connection(|connection| storage::recording_has_redactions(connection, recording_id))
        .map_err(storage_error)?
    {
        tracing::warn!(
            recording_id = %recording_id,
            "ai_processing_start_blocked_redactions"
        );
        return Err(save_readiness_failure(
            storage,
            recording_id,
            "ai_processing_redactions_unsupported",
            "AI processing is blocked for redacted recordings until redaction-aware chunk splitting is implemented.",
        ));
    }
    let audio_plans = audio_chunk_plans(&segments);
    let activity_plans = activity_chunk_plans(&segments, &app_settings);
    let ocr_plans = visual_chunk_plans(&segments, &app_settings, "ocr");
    let vlm_plans = visual_chunk_plans(&segments, &app_settings, "vlm");
    let audio_chunk_count = audio_plans.len();
    let activity_chunk_count = activity_plans.len();
    let ocr_chunk_count = ocr_plans.len();
    let vlm_chunk_count = vlm_plans.len();
    tracing::info!(
        recording_id = %recording_id,
        segment_count = segments.len(),
        audio_chunk_count,
        activity_chunk_count,
        ocr_chunk_count,
        vlm_chunk_count,
        transcription_provider = %app_settings.ai.transcription.provider,
        transcription_model = %app_settings.ai.transcription.local_model_id,
        transcription_backend = %app_settings.ai.transcription.local_backend,
        word_timestamps = app_settings.ai.transcription.word_timestamps_enabled,
        "ai_processing_plan_built"
    );
    if !audio_plans.is_empty() {
        if let Err(error) = check_transcription_readiness(app, storage, &app_settings) {
            tracing::warn!(
                recording_id = %recording_id,
                error_code = %error.code,
                "ai_processing_start_blocked_transcription_readiness"
            );
            return Err(save_readiness_failure(
                storage,
                recording_id,
                &error.code,
                &error.message,
            ));
        }
    }

    if audio_plans.is_empty()
        && activity_plans.is_empty()
        && ocr_plans.is_empty()
        && vlm_plans.is_empty()
    {
        tracing::warn!(
            recording_id = %recording_id,
            "ai_processing_start_blocked_no_enabled_sources"
        );
        return Err(save_readiness_failure(
            storage,
            recording_id,
            "ai_processing_no_enabled_sources",
            "Recording has no enabled audio or activity sources to process.",
        ));
    }

    let job = storage
        .with_connection(|connection| {
            if let Some(existing) = storage::get_latest_processing_job_for_recording(
                connection,
                recording_id,
                AI_PROCESSING_JOB_KIND,
            )? {
                tracing::info!(
                    recording_id = %recording_id,
                    job_id = %existing.id,
                    job_status = %existing.status,
                    "ai_processing_existing_job_found"
                );
                match existing.status.as_str() {
                    "queued" | "running" | "paused" | "succeeded" => return Ok(existing),
                    "failed" | "cancelled" => {
                        return storage::retry_processing_job_chunks(
                            connection,
                            &existing.id,
                            now_ms(),
                        );
                    }
                    _ => {}
                }
            }

            let timestamp_ms = now_ms();
            let job_id = Uuid::now_v7().to_string();
            let job = storage::create_processing_job(
                connection,
                &CreateProcessingJobInput {
                    created_at_ms: timestamp_ms,
                    error: None,
                    id: job_id.clone(),
                    kind: AI_PROCESSING_JOB_KIND.to_string(),
                    recording_id: recording_id.to_string(),
                    status: "queued".to_string(),
                    updated_at_ms: timestamp_ms,
                },
            )?;
            let desktop_track = ensure_audio_track(
                connection,
                recording_id,
                "desktop-audio",
                "Desktop audio",
                timestamp_ms,
            )?;
            let microphone_track = ensure_audio_track(
                connection,
                recording_id,
                "microphone",
                "Microphone",
                timestamp_ms,
            )?;
            for plan in audio_plans {
                let track_id = match plan.source_role.as_str() {
                    "desktop-audio" => desktop_track.id.clone(),
                    "microphone" => microphone_track.id.clone(),
                    _ => return Err(rusqlite::Error::InvalidQuery),
                };
                storage::create_processing_chunk(
                    connection,
                    &CreateProcessingChunkInput {
                        attempt_count: 0,
                        created_at_ms: timestamp_ms,
                        end_offset_ms: Some(plan.end_offset_ms),
                        error_code: None,
                        id: Uuid::now_v7().to_string(),
                        job_id: job_id.clone(),
                        progress: 0.0,
                        recording_id: recording_id.to_string(),
                        result_code: None,
                        segment_id: Some(plan.segment_id),
                        source_kind: "audio".to_string(),
                        start_offset_ms: Some(plan.start_offset_ms),
                        status: "queued".to_string(),
                        track_id: Some(track_id),
                        updated_at_ms: timestamp_ms,
                    },
                )?;
            }
            for plan in activity_plans {
                storage::create_processing_chunk(
                    connection,
                    &CreateProcessingChunkInput {
                        attempt_count: 0,
                        created_at_ms: timestamp_ms,
                        end_offset_ms: Some(plan.end_offset_ms),
                        error_code: None,
                        id: Uuid::now_v7().to_string(),
                        job_id: job_id.clone(),
                        progress: 0.0,
                        recording_id: recording_id.to_string(),
                        result_code: None,
                        segment_id: Some(plan.segment_id),
                        source_kind: "activity".to_string(),
                        start_offset_ms: Some(plan.start_offset_ms),
                        status: "queued".to_string(),
                        track_id: None,
                        updated_at_ms: timestamp_ms,
                    },
                )?;
            }
            for (source_kind, plans) in [("ocr", ocr_plans), ("vlm", vlm_plans)] {
                for plan in plans {
                    storage::create_processing_chunk(
                        connection,
                        &CreateProcessingChunkInput {
                            attempt_count: 0,
                            created_at_ms: timestamp_ms,
                            end_offset_ms: Some(plan.end_offset_ms),
                            error_code: None,
                            id: Uuid::now_v7().to_string(),
                            job_id: job_id.clone(),
                            progress: 0.0,
                            recording_id: recording_id.to_string(),
                            result_code: None,
                            segment_id: Some(plan.segment_id),
                            source_kind: source_kind.to_string(),
                            start_offset_ms: Some(plan.start_offset_ms),
                            status: "queued".to_string(),
                            track_id: None,
                            updated_at_ms: timestamp_ms,
                        },
                    )?;
                }
            }
            save_processing_notification(
                connection,
                ProcessingNotificationInput {
                    actions: &processing_running_actions(&job_id),
                    job_id: Some(&job_id),
                    message: "AI processing is queued for this recording.",
                    recording_id,
                    status: "running",
                    timestamp_ms,
                    title: "AI processing queued",
                },
            )?;
            tracing::info!(
                recording_id = %recording_id,
                job_id = %job_id,
                audio_chunk_count,
                activity_chunk_count,
                ocr_chunk_count,
                vlm_chunk_count,
                "ai_processing_job_created"
            );
            Ok(job)
        })
        .map_err(storage_error)?;
    emit_processing_state(app, recording_id);
    Ok(job)
}

fn check_transcription_readiness<R: tauri::Runtime>(
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

fn save_readiness_failure(
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
