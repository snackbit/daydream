//! Versioned source-time speech processing. Legacy jobs keep their original cuts.
mod execution;
mod input;
mod planning;
mod publication;
mod reference;
mod stitching;
#[cfg(test)]
mod tests;

#[cfg(test)]
mod recall_integration_tests;

use super::*;
use input::{extract_window, source_runs, SourceRun};
use planning::{plan_units, CONTEXT_MS, OWNED_WINDOW_MS};

pub(super) async fn process(
    app: &AppHandle,
    chunk: &ProcessingChunkRow,
    cancel: Arc<AtomicBool>,
    lease: crate::recording_privacy::ProcessingLease,
) -> Result<AudioChunkProcessingOutcome, AiError> {
    let storage = app.state::<AppStorage>();
    let settings = super::super::worker::load_chunk_settings(&storage, chunk)?;
    let input = load_audio_chunk_input(app, chunk)?;
    let track = input.track;
    let runs = source_runs(&storage, chunk, &track)?;
    publication::prune_ineligible_units(&storage, chunk, &track, &runs)?;
    let mut model_path = app
        .path()
        .resolve(
            whisper_backend::vad::MODEL_FILE,
            tauri::path::BaseDirectory::Resource,
        )
        .map_err(|_| {
            ai_error(
                "ai_transcription_vad_model_missing",
                "The bundled speech detector could not be located.",
            )
        })?;
    if cfg!(debug_assertions) && !model_path.is_file() {
        model_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources")
            .join(whisper_backend::vad::MODEL_FILE);
    }
    use sha2::{Digest, Sha256};
    let config = serde_json::to_vec(&settings.ai.transcription).map_err(|_| {
        ai_error(
            "ai_processing_config_invalid",
            "The saved transcription configuration is invalid.",
        )
    })?;
    let fingerprint = hex::encode(Sha256::digest(
        [
            config.as_slice(),
            whisper_backend::vad::MODEL_SHA256.as_bytes(),
            b"speech-units-v2",
        ]
        .concat(),
    ));
    let target = chunk.end_offset_ms.ok_or_else(|| {
        ai_error(
            "ai_processing_chunk_offset_missing",
            "Audio timing is unavailable.",
        )
    })?;
    let start = chunk.start_offset_ms.unwrap_or(0);
    let mut waiting = false;
    for run in runs
        .iter()
        .filter(|run| run.start < target && run.end > start)
    {
        let mut cursor = storage
            .with_connection(|c| {
                let mut cursor = run.start;
                for unit in storage::speech_units(c, &chunk.job_id, &track.id)?
                    .iter()
                    .filter(|unit| unit.start_ms >= run.start && unit.end_ms <= run.end)
                {
                    if unit.start_ms > cursor {
                        break;
                    }
                    cursor = cursor.max(unit.end_ms);
                }
                Ok(cursor)
            })
            .map_err(storage_error)?;
        while cursor < target.min(run.end) {
            check_atomic_cancel(&cancel)?;
            ensure_chunk_active(&storage, chunk)?;
            let scan_start = (cursor - CONTEXT_MS).max(run.start);
            let scan_end = (cursor + OWNED_WINDOW_MS + CONTEXT_MS).min(run.end);
            let path = processing_temp_audio_path(
                app,
                &chunk.job_id,
                &format!("{}-speech-scan", chunk.id),
            )?;
            let temp = TempAudioFile::new(path.clone());
            let run_copy = run.window(scan_start, scan_end);
            let model = model_path.clone();
            let worker_cancel = cancel.clone();
            let worker_lease = lease.clone();
            let min_speech = settings.ai.transcription.min_speech_ms as i64;
            storage
                .with_connection(|c| {
                    storage::set_processing_chunk_phase(c, &chunk.id, "detecting-speech")
                })
                .map_err(storage_error)?;
            let speech = tauri::async_runtime::spawn_blocking(move || {
                let _lease = worker_lease;
                extract_window(&run_copy, scan_start, scan_end, &path, &worker_cancel)?;
                whisper_backend::vad::detect_unpadded_speech_ranges(
                    &model,
                    &path,
                    min_speech,
                    worker_cancel,
                )
                .map_err(whisper_backend_error)
            })
            .await
            .map_err(|_| {
                ai_error(
                    "ai_transcription_vad_worker_failed",
                    "Speech detection worker stopped.",
                )
            })??;
            drop(temp);
            let units = plan_units(
                chunk,
                &track,
                planning::PlanningWindow {
                    cursor,
                    scan_start,
                    scan_end,
                    run_end: run.end,
                    open: run.open,
                },
                &speech,
                &fingerprint,
            );
            let next = units.last().map(|unit| unit.end_ms).unwrap_or(cursor);
            if next == cursor {
                waiting = true;
                break;
            }
            publication::checkpoint(&storage, chunk, |tx| {
                for unit in &units {
                    storage::save_speech_unit(tx, unit)?;
                }
                Ok(true)
            })?;
            cursor = next;
        }
        execution::SpeechWorker {
            app,
            chunk,
            track: &track,
            run,
            settings: &settings,
            fingerprint: &fingerprint,
            cancel: cancel.clone(),
            lease: lease.clone(),
        }
        .process_units(target)
        .await?;
    }
    // An unfinished utterance requeues its owner without occupying a model worker.
    if waiting {
        storage.with_connection(|c|{
            storage::update_processing_chunk_status(c,&chunk.id,"queued",0.0,None,now_ms())?;
            c.execute("UPDATE processing_chunk_runtime SET phase='waiting-for-audio-context',next_retry_at_ms=?2 WHERE chunk_id=?1",rusqlite::params![chunk.id,now_ms()+2000])?;Ok(())
        }).map_err(storage_error)?;
        return Err(ai_error(
            "ai_processing_waiting_for_audio",
            "Waiting for finalized audio context.",
        ));
    }
    Ok(AudioChunkProcessingOutcome { result_code: None })
}
