use super::super::super::*;
use super::readiness::save_readiness_failure;
use rusqlite::OptionalExtension;

pub(crate) fn enqueue_recording_ai_processing<R: tauri::Runtime>(
    app: &AppHandle<R>,
    storage: &AppStorage,
    recording_id: &str,
) -> Result<ProcessingJobRow, AiError> {
    enqueue_processing(app, storage, recording_id, false)
}

fn enqueue_processing<R: tauri::Runtime>(
    app: &AppHandle<R>,
    storage: &AppStorage,
    recording_id: &str,
    append: bool,
) -> Result<ProcessingJobRow, AiError> {
    tracing::info!(recording_id = %recording_id, "ai_processing_start_requested");
    let app_settings = settings::load_app_settings(storage).map_err(settings_error)?;
    let ocr_config_json = capture_ocr_job_config(&app_settings)?;
    let recording = storage
        .with_connection(|connection| storage::get_recording(connection, recording_id))
        .map_err(storage_error)?;
    let input_open = matches!(recording.status.as_str(), "recording" | "paused");
    if !input_open
        && !storage
            .with_connection(|connection| {
                crate::recording_privacy::protection_ready(connection, recording_id)
            })
            .map_err(storage_error)?
    {
        return Err(ai_error(
            "ai_processing_protection_pending",
            "Protected recording spans must be recovered before processing.",
        ));
    }
    if !matches!(
        recording.status.as_str(),
        "completed" | "failed" | "recovered-partial"
    ) && !input_open
    {
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
    let redactions = storage
        .with_connection(|connection| storage::list_redactions(connection, recording_id))
        .map_err(storage_error)?;
    let audio_plans = audio_chunk_plans(&segments, &redactions);
    let activity_plans = activity_chunk_plans(&segments, &app_settings, &redactions);
    let ocr_plans = storage
        .with_connection(|connection| {
            visual_source_chunk_plans(connection, &segments, &app_settings, &redactions, "ocr")
        })
        .map_err(storage_error)?;
    let vlm_plans = storage
        .with_connection(|connection| {
            visual_source_chunk_plans(connection, &segments, &app_settings, &redactions, "vlm")
        })
        .map_err(storage_error)?;
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
        .with_connection_mut(|database| {
            let transaction = database.transaction()?;
            let connection = &transaction;
            let existing = storage::get_latest_processing_job_for_recording(
                connection,
                recording_id,
                AI_PROCESSING_JOB_KIND,
            )?;
            if let Some(existing) = &existing {
                tracing::info!(
                    recording_id = %recording_id,
                    job_id = %existing.id,
                    job_status = %existing.status,
                    "ai_processing_existing_job_found"
                );
                if !append
                    && !input_open
                    && !storage::processing_job_input_open(connection, &existing.id)?
                {
                    match existing.status.as_str() {
                        "queued" | "running" | "paused" | "succeeded" => {
                            return Ok(existing.clone())
                        }
                        "failed" | "cancelled" | "completed-with-errors" => {
                            let retried = storage::retry_processing_job_chunks(
                                connection,
                                &existing.id,
                                now_ms(),
                            )?;
                            transaction.commit()?;
                            return Ok(retried);
                        }
                        _ => {}
                    }
                }
                // Automatic fragment admission must not undo a user's cancel.
                if existing.status == "cancelled" {
                    if !input_open {
                        connection.execute(
                            "UPDATE processing_job_runtime SET input_open=0 WHERE job_id=?1",
                            [&existing.id],
                        )?;
                        transaction.commit()?;
                    }
                    return Ok(existing.clone());
                }
            }

            let timestamp_ms = now_ms();
            let job_id = existing
                .as_ref()
                .map(|job| job.id.clone())
                .unwrap_or_else(|| Uuid::now_v7().to_string());
            let job = if let Some(existing) = existing {
                existing
            } else {
                storage::create_processing_job(
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
                )?
            };
            storage::save_processing_job_ocr_config(connection, &job.id, &ocr_config_json)?;
            let config =
                serde_json::to_string(&app_settings).map_err(|_| rusqlite::Error::InvalidQuery)?;
            storage::set_processing_job_runtime(connection, &job.id, input_open, &config)?;
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
                create_chunk_once(
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
                create_chunk_once(
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
                    create_chunk_once(
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
                            track_id: plan.track_id,
                            updated_at_ms: timestamp_ms,
                        },
                    )?;
                }
            }
            let job = if matches!(job.status.as_str(), "running" | "paused") {
                job
            } else {
                storage::update_processing_job_status(
                    connection,
                    &job.id,
                    "queued",
                    None,
                    timestamp_ms,
                )?
            };
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
            transaction.commit()?;
            Ok(job)
        })
        .map_err(storage_error)?;
    emit_processing_state(app, recording_id);
    Ok(job)
}

fn create_chunk_once(
    connection: &rusqlite::Connection,
    input: &CreateProcessingChunkInput,
) -> rusqlite::Result<()> {
    let exists:bool=connection.query_row("SELECT EXISTS(SELECT 1 FROM processing_chunks WHERE job_id=?1 AND segment_id IS ?2 AND source_kind=?3 AND track_id IS ?4 AND start_offset_ms IS ?5 AND end_offset_ms IS ?6)",rusqlite::params![input.job_id,input.segment_id,input.source_kind,input.track_id,input.start_offset_ms,input.end_offset_ms],|row|row.get(0))?;
    if exists {
        return Ok(());
    }
    if !crate::recording_privacy::range_protection_ready(
        connection,
        &input.recording_id,
        input.start_offset_ms.unwrap_or(0),
        input.end_offset_ms.unwrap_or(i64::MAX),
    )? {
        return Ok(());
    }
    storage::create_processing_chunk(connection, input)?;
    Ok(())
}

pub(crate) fn admit_live_recording<R: tauri::Runtime>(
    app: &AppHandle<R>,
    storage: &AppStorage,
    id: &str,
) -> Result<(), AiError> {
    let settings = settings::load_app_settings(storage).map_err(settings_error)?;
    if !settings.ai.processing.auto_start_after_recording
        || settings.ai.processing.timing != "during-recording"
    {
        return Ok(());
    }
    let needed=storage.with_connection(|connection|{
        let recording=storage::get_recording(connection,id)?;
        connection.execute("UPDATE recording_live_state SET classified_through_ms=MAX(classified_through_ms,?2) WHERE recording_id=?1 AND classifier_required=0",rusqlite::params![id,now_ms().saturating_sub(recording.started_at_ms.unwrap_or(now_ms()))])?;
        let job=storage::get_latest_processing_job_for_recording(connection,id,AI_PROCESSING_JOB_KIND)?;
        if job.as_ref().is_some_and(|job|job.status=="cancelled"){return Ok(false);}
        let watermark:Option<i64>=connection.query_row("SELECT classified_through_ms FROM recording_live_state WHERE recording_id=?1",[id],|row|row.get(0)).optional()?;
        let segments=storage::list_recording_segments(connection,id)?;
        for segment in &segments {
            if segment.state!="finalized"{continue;}
            let end=segment.ended_at_ms.unwrap_or(i64::MAX).saturating_sub(recording.started_at_ms.unwrap_or(0));
            if watermark.is_none_or(|watermark|end>watermark){continue;}
            let admitted:bool=connection.query_row("SELECT EXISTS(SELECT 1 FROM processing_chunks WHERE recording_id=?1 AND segment_id=?2)",rusqlite::params![id,segment.id],|row|row.get(0))?;
            if !admitted{return Ok(true);}
        }
        Ok(false)
    }).map_err(storage_error)?;
    if needed {
        enqueue_processing(app, storage, id, true)?;
    }
    Ok(())
}

pub(crate) fn finish_live_recording<R: tauri::Runtime>(
    app: &AppHandle<R>,
    storage: &AppStorage,
    id: &str,
) -> Result<bool, AiError> {
    let had_open = storage
        .with_connection(|connection| {
            let job = storage::get_latest_processing_job_for_recording(
                connection,
                id,
                AI_PROCESSING_JOB_KIND,
            )?;
            if let Some(job) = job {
                storage::processing_job_input_open(connection, &job.id)
            } else {
                Ok(false)
            }
        })
        .map_err(storage_error)?;
    if !had_open {
        return Ok(false);
    }
    let result = enqueue_processing(app, storage, id, true);
    storage.with_connection(|connection|{
        connection.execute("UPDATE processing_job_runtime SET input_open=0 WHERE job_id IN(SELECT id FROM processing_jobs WHERE recording_id=?1)",[id])?;
        connection.execute("UPDATE processing_chunk_runtime SET next_retry_at_ms=NULL,phase='queued' WHERE phase='waiting-for-audio-context' AND chunk_id IN(SELECT id FROM processing_chunks WHERE recording_id=?1)",[id])?;Ok(())
    }).map_err(storage_error)?;
    result?;
    Ok(true)
}
