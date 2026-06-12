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
    match ProcessingChunkSource::from_chunk(&chunk)? {
        ProcessingChunkSource::Audio => process_audio_chunk(app, job_id, chunk).await,
        ProcessingChunkSource::Activity => process_activity_chunk(app, job_id, chunk).await,
        ProcessingChunkSource::Visual => process_visual_chunk(app, job_id, chunk).await,
    }
}

async fn process_audio_chunk(
    app: &AppHandle,
    job_id: &str,
    chunk: ProcessingChunkRow,
) -> Result<(), AiError> {
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
        process_audio_chunk_inner(app, &chunk, cancel)
    })
    .await;

    match result {
        Ok(outcome) => {
            context.mark_succeeded(outcome.result_code.as_deref())?;
            tracing::info!(
                recording_id = %chunk.recording_id,
                job_id = %job_id,
                chunk_id = %chunk.id,
                track_id = chunk.track_id.as_deref().unwrap_or(""),
                "ai_processing_audio_chunk_succeeded"
            );
            context.emit_processing_state();
            Ok(())
        }
        Err(error) if error.code == "ai_transcription_cancelled" => {
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
        process_activity_chunk_inner(app, &chunk, cancel)
    })
    .await;

    match result {
        Ok(()) => {
            context.mark_succeeded(None)?;
            tracing::info!(
                recording_id = %chunk.recording_id,
                job_id = %job_id,
                chunk_id = %chunk.id,
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
) -> Result<(), AiError> {
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
        process_visual_chunk_inner(app, &chunk, cancel)
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
    let result = operation(cancel).await;
    monitor.abort();
    result
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
