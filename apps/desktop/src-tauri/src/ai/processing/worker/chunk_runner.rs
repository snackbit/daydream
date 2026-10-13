use super::super::super::*;
use super::{
    cancel::spawn_job_cancel_monitor, chunk_context::ProcessingChunkContext,
    dispatch::ProcessingChunkSource,
};

pub(super) async fn process_processing_chunk(
    app: &AppHandle,
    job_id: &str,
    chunk: ProcessingChunkRow,
) -> Result<(), AiError> {
    let _lease = crate::recording_privacy::processing_lease(&chunk.recording_id).await;
    if let Err(error) = ensure_chunk_active(&app.state::<AppStorage>(), &chunk) {
        if error.code != "ai_processing_cancelled" {
            return Err(error);
        }
        let context = ProcessingChunkContext::new(app, job_id, &chunk);
        let job = context.current_job()?;
        if job.status == "paused" {
            context.requeue_paused()?;
            return Err(ai_error(
                "ai_processing_paused",
                "Processing paused before this section started.",
            ));
        }
        if job.status == "cancelled" {
            context.mark_cancelled()?;
            return Err(ai_error(
                "ai_processing_cancelled",
                "Processing was cancelled.",
            ));
        }
        context.mark_cancelled()?;
        context.emit_processing_state();
        return Ok(());
    }
    let source = match ProcessingChunkSource::from_chunk(&chunk) {
        Ok(source) => source,
        Err(error) => {
            ProcessingChunkContext::new(app, job_id, &chunk).mark_failed(&error.code)?;
            return Err(error);
        }
    };
    match source {
        ProcessingChunkSource::Audio => process_audio_chunk(app, job_id, chunk, _lease).await,
        ProcessingChunkSource::Activity => process_activity_chunk(app, job_id, chunk).await,
        ProcessingChunkSource::Visual => process_visual_chunk(app, job_id, chunk, _lease).await,
        ProcessingChunkSource::Waveform => {
            let context = ProcessingChunkContext::new(app, job_id, &chunk);
            let result = run_with_cancel_monitor(app, job_id, |cancel| {
                let app = app.clone();
                let chunk = chunk.clone();
                let lease = _lease.clone();
                async move {
                    tauri::async_runtime::spawn_blocking(move || {
                        let _lease = lease;
                        crate::library::waveforms::jobs::prepare_chunk(
                            &app.state::<AppStorage>(),
                            &chunk,
                            &cancel,
                        )
                        .map_err(|code| ai_error(&code, "Waveform preparation could not finish."))
                    })
                    .await
                    .map_err(|_| {
                        ai_error("ai_waveform_generation_failed", "Waveform worker stopped.")
                    })?
                }
            })
            .await;
            match result {
                Ok(()) => context.mark_succeeded(None)?,
                Err(error) if error.code == "ai_processing_cancelled" => {
                    return handle_processing_cancel_signal(
                        &context, &chunk, job_id, error, "waveform",
                    )
                }
                Err(error) => {
                    context.mark_failed(&error.code)?;
                    context.emit_processing_state();
                    return Err(error);
                }
            }
            context.emit_processing_state();
            Ok(())
        }
    }
}

async fn process_audio_chunk(
    app: &AppHandle,
    job_id: &str,
    chunk: ProcessingChunkRow,
    processing_lease: Arc<tokio::sync::OwnedRwLockReadGuard<()>>,
) -> Result<(), AiError> {
    let started = Instant::now();
    let context = ProcessingChunkContext::new(app, job_id, &chunk);
    tracing::info!(
        recording_id = %chunk.recording_id,
        job_id = %job_id,
        chunk_id = %chunk.id,
        track_id = chunk.track_id.as_deref().unwrap_or(""),
        segment_id = chunk.segment_id.as_deref().unwrap_or(""),
        start_offset_ms = ?chunk.start_offset_ms,
        end_offset_ms = ?chunk.end_offset_ms,
        attempt_count = chunk.attempt_count,
        "ai_processing_audio_chunk_started"
    );
    context.emit_processing_state();
    let result = run_with_cancel_monitor(app, job_id, |cancel| {
        process_audio_chunk_inner(app, &chunk, cancel, processing_lease.clone())
    })
    .await;

    match result {
        Err(error) if error.code == "ai_processing_waiting_for_audio" => {
            context.emit_processing_state();
            Ok(())
        }
        Ok(outcome) => {
            context.mark_succeeded(outcome.result_code.as_deref())?;
            tracing::info!(
                recording_id = %chunk.recording_id,
                job_id = %job_id,
                chunk_id = %chunk.id,
                track_id = chunk.track_id.as_deref().unwrap_or(""),
                elapsed_ms = elapsed_ms(started),
                "ai_processing_audio_chunk_succeeded"
            );
            context.emit_processing_state();
            Ok(())
        }
        Err(error)
            if matches!(
                error.code.as_str(),
                "ai_transcription_cancelled" | "ai_processing_cancelled"
            ) =>
        {
            handle_audio_cancel_signal(&context, &chunk, job_id, error)
        }
        Err(error) => {
            context.mark_failed(&error.code)?;
            tracing::warn!(
                recording_id = %chunk.recording_id,
                job_id = %job_id,
                chunk_id = %chunk.id,
                track_id = chunk.track_id.as_deref().unwrap_or(""),
                error_code = %error.code,
                elapsed_ms = elapsed_ms(started),
                "ai_processing_audio_chunk_failed"
            );
            context.emit_processing_state();
            Err(error)
        }
    }
}

async fn process_activity_chunk(
    app: &AppHandle,
    job_id: &str,
    chunk: ProcessingChunkRow,
) -> Result<(), AiError> {
    let started = Instant::now();
    let context = ProcessingChunkContext::new(app, job_id, &chunk);
    tracing::info!(
        recording_id = %chunk.recording_id,
        job_id = %job_id,
        chunk_id = %chunk.id,
        segment_id = chunk.segment_id.as_deref().unwrap_or(""),
        start_offset_ms = ?chunk.start_offset_ms,
        end_offset_ms = ?chunk.end_offset_ms,
        attempt_count = chunk.attempt_count,
        "ai_processing_activity_chunk_started"
    );
    context.emit_processing_state();
    let result = run_with_cancel_monitor(app, job_id, |cancel| {
        let app = app.clone();
        let chunk = chunk.clone();
        async move {
            tauri::async_runtime::spawn_blocking(move || {
                process_activity_chunk_inner(&app, &chunk, cancel)
            })
            .await
            .map_err(|_| {
                ai_error(
                    "ai_processing_activity_worker_failed",
                    "Activity processing worker failed.",
                )
            })?
        }
    })
    .await;

    match result {
        Ok(()) => {
            context.mark_succeeded(None)?;
            tracing::info!(
                recording_id = %chunk.recording_id,
                job_id = %job_id,
                chunk_id = %chunk.id,
                elapsed_ms = elapsed_ms(started),
                "ai_processing_activity_chunk_succeeded"
            );
            context.emit_processing_state();
            Ok(())
        }
        Err(error) if error.code == "ai_processing_cancelled" => {
            handle_processing_cancel_signal(&context, &chunk, job_id, error, "activity")
        }
        Err(error) => {
            context.mark_failed(&error.code)?;
            tracing::warn!(
                recording_id = %chunk.recording_id,
                job_id = %job_id,
                chunk_id = %chunk.id,
                error_code = %error.code,
                elapsed_ms = elapsed_ms(started),
                "ai_processing_activity_chunk_failed"
            );
            context.emit_processing_state();
            Err(error)
        }
    }
}

async fn process_visual_chunk(
    app: &AppHandle,
    job_id: &str,
    chunk: ProcessingChunkRow,
    lease: crate::recording_privacy::ProcessingLease,
) -> Result<(), AiError> {
    let started = Instant::now();
    let context = ProcessingChunkContext::new(app, job_id, &chunk);
    tracing::info!(
        recording_id = %chunk.recording_id,
        job_id = %job_id,
        chunk_id = %chunk.id,
        source_kind = %chunk.source_kind,
        segment_id = chunk.segment_id.as_deref().unwrap_or(""),
        start_offset_ms = ?chunk.start_offset_ms,
        end_offset_ms = ?chunk.end_offset_ms,
        attempt_count = chunk.attempt_count,
        "ai_processing_visual_chunk_started"
    );
    context.emit_processing_state();
    let result = run_with_cancel_monitor(app, job_id, |cancel| {
        process_visual_chunk_inner(app, &chunk, cancel, lease.clone())
    })
    .await;

    match result {
        Ok(()) => {
            context.mark_succeeded(None)?;
            tracing::info!(
                recording_id = %chunk.recording_id,
                job_id = %job_id,
                chunk_id = %chunk.id,
                source_kind = %chunk.source_kind,
                elapsed_ms = elapsed_ms(started),
                "ai_processing_visual_chunk_succeeded"
            );
            context.emit_processing_state();
            Ok(())
        }
        Err(error) if error.code == "ai_processing_cancelled" => {
            handle_processing_cancel_signal(&context, &chunk, job_id, error, "visual")
        }
        Err(error) => {
            context.mark_failed(&error.code)?;
            tracing::warn!(
                recording_id = %chunk.recording_id,
                job_id = %job_id,
                chunk_id = %chunk.id,
                source_kind = %chunk.source_kind,
                error_code = %error.code,
                elapsed_ms = elapsed_ms(started),
                "ai_processing_visual_chunk_failed"
            );
            context.emit_processing_state();
            Err(error)
        }
    }
}

async fn run_with_cancel_monitor<T, Fut>(
    app: &AppHandle,
    job_id: &str,
    operation: impl FnOnce(Arc<AtomicBool>) -> Fut,
) -> Result<T, AiError>
where
    Fut: std::future::Future<Output = Result<T, AiError>>,
{
    let cancel = Arc::new(AtomicBool::new(false));
    let monitor = spawn_job_cancel_monitor(app.clone(), job_id.to_string(), cancel.clone());
    let guard = CancelMonitorGuard {
        cancel: cancel.clone(),
        monitor,
    };
    let result = operation(cancel).await;
    drop(guard);
    result
}

struct CancelMonitorGuard {
    cancel: Arc<AtomicBool>,
    monitor: tauri::async_runtime::JoinHandle<()>,
}
impl Drop for CancelMonitorGuard {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::SeqCst);
        self.monitor.abort();
    }
}

fn handle_audio_cancel_signal(
    context: &ProcessingChunkContext<'_>,
    chunk: &ProcessingChunkRow,
    job_id: &str,
    error: AiError,
) -> Result<(), AiError> {
    let job = context.current_job()?;
    match job.status.as_str() {
        "paused" => {
            context.requeue_paused()?;
            tracing::info!(
                recording_id = %chunk.recording_id,
                job_id = %job_id,
                chunk_id = %chunk.id,
                track_id = chunk.track_id.as_deref().unwrap_or(""),
                "ai_processing_audio_chunk_requeued_paused"
            );
            context.emit_processing_state();
            Err(ai_error(
                "ai_processing_paused",
                "AI processing was paused.",
            ))
        }
        "cancelled" => {
            context.mark_cancelled()?;
            tracing::info!(
                recording_id = %chunk.recording_id,
                job_id = %job_id,
                chunk_id = %chunk.id,
                track_id = chunk.track_id.as_deref().unwrap_or(""),
                "ai_processing_audio_chunk_cancelled"
            );
            context.emit_processing_state();
            Err(ai_error(
                "ai_processing_cancelled",
                "AI processing was cancelled.",
            ))
        }
        _ => {
            context.mark_failed(&error.code)?;
            tracing::warn!(
                recording_id = %chunk.recording_id,
                job_id = %job_id,
                chunk_id = %chunk.id,
                track_id = chunk.track_id.as_deref().unwrap_or(""),
                error_code = %error.code,
                job_status = %job.status,
                "ai_processing_audio_chunk_failed_after_cancel_signal"
            );
            context.emit_processing_state();
            Err(error)
        }
    }
}

fn handle_processing_cancel_signal(
    context: &ProcessingChunkContext<'_>,
    chunk: &ProcessingChunkRow,
    job_id: &str,
    error: AiError,
    source_label: &str,
) -> Result<(), AiError> {
    let job = context.current_job()?;
    match job.status.as_str() {
        "paused" => {
            context.requeue_paused()?;
            if source_label == "activity" {
                tracing::info!(
                    recording_id = %chunk.recording_id,
                    job_id = %job_id,
                    chunk_id = %chunk.id,
                    "ai_processing_activity_chunk_requeued_paused"
                );
            }
            context.emit_processing_state();
            Err(ai_error(
                "ai_processing_paused",
                "AI processing was paused.",
            ))
        }
        "cancelled" => {
            context.mark_cancelled()?;
            if source_label == "activity" {
                tracing::info!(
                    recording_id = %chunk.recording_id,
                    job_id = %job_id,
                    chunk_id = %chunk.id,
                    "ai_processing_activity_chunk_cancelled"
                );
            }
            context.emit_processing_state();
            Err(error)
        }
        _ => Err(error),
    }
}
