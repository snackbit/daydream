use super::super::super::*;
use super::{
    chunk_runner::process_processing_chunk,
    completion::{fail_processing_job, finish_processing_job},
    dispatch::claim_with_capacity,
    job_state::{handle_current_job_status, JobStatusAction},
};

pub(super) async fn process_recording_ai_job(
    app: AppHandle,
    job_id: String,
    ownership: Arc<super::ownership::WorkerOwnership>,
) -> Result<(), AiError> {
    let started = Instant::now();
    let storage = app.state::<AppStorage>();
    let job = storage
        .with_connection(|connection| storage::get_processing_job(connection, &job_id))
        .map_err(storage_error)?
        .ok_or_else(|| ai_error("ai_processing_job_missing", "Processing job is missing."))?;
    tracing::info!(
        job_id = %job.id,
        recording_id = %job.recording_id,
        "AI processing job started."
    );
    if job.kind != "waveform" {
        save_job_running_notification(&storage, &job)?;
    }
    let max_parallel_chunks = if job.kind == "waveform" {
        1
    } else {
        let app_settings = settings::load_app_settings(&storage).map_err(settings_error)?;
        app_settings
            .ai
            .processing
            .max_parallel_chunks_per_job
            .max(1)
            .min(
                if app_settings.ai.processing.resource_mode == "background" {
                    2
                } else {
                    u32::MAX
                },
            ) as usize
    };
    let mut active_chunks = futures_util::stream::FuturesUnordered::new();
    let mut local_active = 0usize;
    let mut remote_active = 0usize;

    loop {
        match handle_current_job_status(&app, &storage, &job_id)? {
            JobStatusAction::Continue => {}
            JobStatusAction::Stop => {
                while let Some(_remaining) = active_chunks.next().await {}
                return Ok(());
            }
        }

        while local_active < max_parallel_chunks || remote_active < 1 {
            let Some(chunk) = claim_with_capacity(
                &storage,
                &job_id,
                local_active < max_parallel_chunks,
                remote_active < 1,
            )?
            else {
                break;
            };
            let remote = chunk.source_kind == "vlm";
            if remote {
                remote_active += 1;
            } else {
                local_active += 1;
            }

            let app = app.clone();
            let job_id = job_id.clone();
            let ownership = ownership.clone();
            active_chunks.push(async move {
                let worker_app=app.clone();let worker_id=job_id.clone();let worker_chunk=chunk.clone();
                let result=match tauri::async_runtime::spawn(async move {let _ownership=ownership;process_processing_chunk(&worker_app,&worker_id,worker_chunk).await}).await {
                    Ok(result)=>result,
                    Err(_)=>{
                        super::chunk_context::ProcessingChunkContext::new(&app,&job_id,&chunk).mark_failed("ai_processing_chunk_worker_failed")?;
                        emit_processing_state(&app,&chunk.recording_id);
                        Err(ai_error("ai_processing_chunk_worker_failed","A processing worker stopped unexpectedly. Other sections will continue."))
                    }
                };
                Ok::<_,AiError>((remote,result))
            });
        }

        if active_chunks.is_empty() {
            finish_processing_job(&app, &storage, &job_id)?;
            let current = storage
                .with_connection(|connection| storage::get_processing_job(connection, &job_id))
                .map_err(storage_error)?
                .ok_or_else(|| {
                    ai_error("ai_processing_job_missing", "Processing job is missing.")
                })?;
            if current.status == "running" {
                // Delayed retries and an open recording release worker capacity.
                // The scheduler wakes jobs only when useful work is ready.
                storage
                    .with_connection(|connection| {
                        storage::update_processing_job_status(
                            connection,
                            &job_id,
                            "queued",
                            None,
                            now_ms(),
                        )
                    })
                    .map_err(storage_error)?;
                crate::ai::results::emit_processing_state_with_source(
                    &app,
                    &job.recording_id,
                    (job.kind == "waveform").then_some("waveform"),
                );
                return Ok(());
            }
            tracing::info!(
                job_id = %job.id,
                recording_id = %job.recording_id,
                elapsed_ms = elapsed_ms(started),
                "ai_processing_job_finished"
            );
            super::super::discard_visual_cache(std::slice::from_ref(&job_id));
            return Ok(());
        }

        if let Some(result) = active_chunks.next().await {
            let (remote, result) = result?;
            if remote {
                remote_active -= 1;
            } else {
                local_active -= 1;
            }
            match result {
                Ok(()) => {}
                Err(error)
                    if matches!(
                        error.code.as_str(),
                        "ai_processing_paused" | "ai_processing_cancelled"
                    ) =>
                {
                    while let Some(_remaining) = active_chunks.next().await {}
                    return Ok(());
                }
                Err(error) => {
                    tracing::warn!(
                        job_id = %job.id,
                        recording_id = %job.recording_id,
                        error_code = %error.code,
                        elapsed_ms = elapsed_ms(started),
                        "ai_processing_job_chunk_failed"
                    );
                    if error.code == "ai_storage_failed"
                        || error.code == "ai_processing_job_missing"
                    {
                        fail_processing_job(&app, &storage, &job_id, &error.code)?;
                        while let Some(_remaining) = active_chunks.next().await {}
                        return Err(error);
                    }
                    // Source errors belong to their persisted chunks. Keep
                    // claiming independent work; aggregation owns completion.
                }
            }
        }
    }
}
