use super::super::*;

mod chunk_input;
mod detection_context;
mod extraction;
mod ranges;
mod transcript_persistence;

use chunk_input::load_audio_chunk_input;
use extraction::extract_audio_stream;
use ranges::{audio_path_for_speech_range, plan_submitted_transcription_ranges};
use transcript_persistence::{persist_speech_range, SpeechRangeResult};

#[cfg(test)]
pub(in crate::ai) use extraction::ffmpeg_audio_extract_args;
#[cfg(test)]
pub(in crate::ai) use transcript_persistence::{
    persist_transcriptions_for_chunk, transcript_segment_offsets_ms,
    transcription_words_for_segment, PendingTranscription,
};

pub(in crate::ai) async fn process_audio_chunk_inner(
    app: &AppHandle,
    chunk: &ProcessingChunkRow,
    cancel: Arc<AtomicBool>,
    processing_lease: Arc<tokio::sync::OwnedRwLockReadGuard<()>>,
) -> Result<AudioChunkProcessingOutcome, AiError> {
    let storage = app.state::<AppStorage>();
    ensure_chunk_active(&storage, chunk)?;
    let app_settings = super::worker::load_chunk_settings(&storage, chunk)?;
    let provider_name = app_settings.ai.transcription.provider.clone();
    check_atomic_cancel(&cancel)?;

    let input = load_audio_chunk_input(app, chunk)?;
    let temp_audio = TempAudioFile::new(processing_temp_audio_path(app, &chunk.job_id, &chunk.id)?);
    tracing::info!(
        recording_id = %chunk.recording_id,
        job_id = %chunk.job_id,
        chunk_id = %chunk.id,
        track_id = %input.track.id,
        source_role = %input.track.kind,
        stream_index = input.stream_index,
        slice_start_ms = input.slice_start_ms,
        slice_duration_ms = input.slice_duration_ms,
        "ai_processing_audio_extract_started"
    );
    let extraction_input = input.clone();
    let extraction_path = temp_audio.path().to_path_buf();
    let extraction_cancel = cancel.clone();
    let extraction_lease = processing_lease.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _lease = extraction_lease;
        extract_audio_stream(
            &extraction_input.segment.file_path,
            extraction_input.stream_index,
            extraction_input.slice_start_ms,
            extraction_input.slice_duration_ms,
            &extraction_path,
            &extraction_cancel,
        )
    })
    .await
    .map_err(|_| {
        ai_error(
            "ai_processing_audio_extract_worker_failed",
            "Audio extraction worker failed.",
        )
    })??;
    let temp_audio_bytes = fs::metadata(temp_audio.path())
        .ok()
        .map(|metadata| metadata.len())
        .unwrap_or_default();
    tracing::info!(
        recording_id = %chunk.recording_id,
        job_id = %chunk.job_id,
        chunk_id = %chunk.id,
        track_id = %input.track.id,
        source_role = %input.track.kind,
        stream_index = input.stream_index,
        slice_duration_ms = input.slice_duration_ms,
        temp_audio_bytes,
        "ai_processing_audio_extract_completed"
    );

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
    storage
        .with_connection(|connection| {
            storage::set_processing_chunk_phase(connection, &chunk.id, "detecting-speech")
        })
        .map_err(storage_error)?;
    emit_processing_state(app, &chunk.recording_id);
    let audio_path = temp_audio.path().to_path_buf();
    let context_app = app.clone();
    let context_chunk = chunk.clone();
    let context_input = input.clone();
    let context_cancel = cancel.clone();
    let context_lease = processing_lease.clone();
    let config = whisper_backend::SilenceGateConfig {
        merge_gap_ms: app_settings.ai.transcription.merge_gap_ms.into(),
        min_speech_ms: app_settings.ai.transcription.min_speech_ms.into(),
        silence_threshold_db: app_settings.ai.transcription.silence_threshold_db,
    };
    let gate_enabled = app_settings.ai.transcription.silence_gate_enabled;
    let duration = input.slice_duration_ms;
    let vad_cancel = cancel.clone();
    let (mut transcription_ranges, context) = tauri::async_runtime::spawn_blocking(move || {
        let _lease = context_lease;
        let context = detection_context::load_context(
            &context_app,
            &context_chunk,
            &context_input,
            &audio_path,
            &context_cancel,
        )?;
        let path = context
            .as_ref()
            .map(|context| context.audio.path())
            .unwrap_or(&audio_path);
        let prefix = context
            .as_ref()
            .map(|context| context.prefix_ms)
            .unwrap_or(0);
        let ranges = plan_submitted_transcription_ranges(
            &model_path,
            path,
            duration,
            prefix,
            gate_enabled,
            config,
            vad_cancel,
        )?;
        Ok::<_, AiError>((ranges, context))
    })
    .await
    .map_err(|_| {
        ai_error(
            "ai_transcription_vad_worker_failed",
            "Speech detection worker failed.",
        )
    })??;
    let prefix = context
        .as_ref()
        .map(|context| context.prefix_ms)
        .unwrap_or(0);
    let full_duration = context
        .as_ref()
        .map(|context| context.duration_ms)
        .unwrap_or(input.slice_duration_ms);
    let selection = super::worker::load_unit_selection(&storage, &chunk.id)?;
    if let Some(targets) = &selection.audio_ranges {
        transcription_ranges.retain(|range| {
            targets.iter().any(|(start, end)| {
                range.end_ms - prefix > *start && range.start_ms - prefix < *end
            })
        });
    }
    use sha2::{Digest, Sha256};
    let config_bytes = serde_json::to_vec(&app_settings.ai.transcription).map_err(|_| {
        ai_error(
            "ai_processing_config_invalid",
            "Transcription configuration is invalid.",
        )
    })?;
    let fingerprint = hex::encode(Sha256::digest(
        [
            config_bytes.as_slice(),
            whisper_backend::vad::MODEL_SHA256.as_bytes(),
        ]
        .concat(),
    ));
    storage
        .with_connection(|connection| {
            storage::set_processing_chunk_total(
                connection,
                &chunk.id,
                transcription_ranges.len() as i64,
            )
        })
        .map_err(storage_error)?;

    storage
        .with_connection(|connection| {
            let checkpoints: i64 = connection.query_row(
                "SELECT COUNT(*) FROM processing_audio_ranges WHERE chunk_id=?1",
                [&chunk.id],
                |row| row.get(0),
            )?;
            if checkpoints > 0 {
                return Ok(0);
            }
            storage::delete_transcript_output_for_chunks(
                connection,
                std::slice::from_ref(&chunk.id),
            )
        })
        .map_err(storage_error)?;

    if transcription_ranges.is_empty() {
        ensure_chunk_active(&storage, chunk)?;
        if let Some(targets) = &selection.audio_ranges {
            storage
                .with_connection(|connection| {
                    for (start, end) in targets {
                        storage::save_audio_range_checkpoint(
                            connection,
                            &chunk.id,
                            *start,
                            *end,
                            &fingerprint,
                            None,
                        )?;
                    }
                    Ok(())
                })
                .map_err(storage_error)?;
        }
        tracing::info!(
            recording_id = %chunk.recording_id,
            job_id = %chunk.job_id,
            chunk_id = %chunk.id,
            track_id = %input.track.id,
            source_role = %input.track.kind,
            silence_threshold_db = app_settings.ai.transcription.silence_threshold_db,
            min_speech_ms = app_settings.ai.transcription.min_speech_ms,
            "ai_processing_audio_chunk_skipped_silence"
        );
        return Ok(AudioChunkProcessingOutcome {
            result_code: Some(TRANSCRIPTION_RESULT_SKIPPED_SILENCE.to_string()),
        });
    }

    super::worker::check_transcription_readiness(app, &storage, &app_settings)?;
    let model = selected_transcription_model(&app_settings)?;
    let provider = app.state::<TranscriptionProviderManager>();
    storage
        .with_connection(|connection| {
            storage::set_processing_chunk_phase(connection, &chunk.id, "transcribing")
        })
        .map_err(storage_error)?;

    tracing::info!(
        chunk_id = %chunk.id,
        job_id = %chunk.job_id,
        model = %model,
        provider = %provider_name,
        backend = %app_settings.ai.transcription.local_backend,
        speech_range_count = transcription_ranges.len(),
        "AI transcription started."
    );
    let mut first_failure = None;
    for (range_index, range) in transcription_ranges.iter().enumerate() {
        let checkpoint = whisper_backend::SpeechRange {
            start_ms: (range.start_ms - prefix).max(0),
            end_ms: (range.end_ms - prefix).min(input.slice_duration_ms),
        };
        check_atomic_cancel(&cancel)?;
        ensure_chunk_active(&storage, chunk)?;
        if storage
            .with_connection(|connection| {
                storage::audio_range_succeeded(
                    connection,
                    &chunk.id,
                    checkpoint.start_ms,
                    checkpoint.end_ms,
                    &fingerprint,
                )
            })
            .map_err(storage_error)?
        {
            continue;
        }
        let range_app = app.clone();
        let range_chunk = chunk.clone();
        let range_copy = *range;
        let range_path = context
            .as_ref()
            .map(|context| context.audio.path())
            .unwrap_or(temp_audio.path())
            .to_path_buf();
        let range_cancel = cancel.clone();
        let range_lease = processing_lease.clone();
        let range_audio = tauri::async_runtime::spawn_blocking(move || {
            let _lease = range_lease;
            audio_path_for_speech_range(
                &range_app,
                &range_chunk,
                full_duration,
                &range_copy,
                range_index,
                &range_path,
                &range_cancel,
            )
        })
        .await
        .map_err(|_| {
            ai_error(
                "ai_processing_audio_extract_worker_failed",
                "Speech-window extraction failed.",
            )
        })??;
        ensure_chunk_active(&storage, chunk)?;
        let transcription = provider
            .transcribe(
                app,
                &storage,
                TranscriptionRequest {
                    processing_lease: Some(processing_lease.clone()),
                    audio_path: range_audio.path,
                    cancel: cancel.clone(),
                    language: app_settings.ai.transcription.language.clone(),
                    max_initial_timestamp_seconds: app_settings
                        .ai
                        .transcription
                        .max_initial_timestamp_seconds,
                    model: model.clone(),
                    timeout: TRANSCRIPTION_TIMEOUT,
                    word_timestamps: app_settings.ai.transcription.word_timestamps_enabled,
                },
                &app_settings,
            )
            .await
            .map_err(transcription_error);
        let transcription = match transcription {
            Ok(value) => value,
            Err(error) => {
                if matches!(
                    error.code.as_str(),
                    "ai_transcription_cancelled" | "ai_processing_cancelled" | "ai_storage_failed"
                ) {
                    return Err(error);
                }
                ensure_chunk_active(&storage, chunk)?;
                storage
                    .with_connection(|connection| {
                        storage::save_audio_range_checkpoint(
                            connection,
                            &chunk.id,
                            checkpoint.start_ms,
                            checkpoint.end_ms,
                            &fingerprint,
                            Some(&error.code),
                        )
                    })
                    .map_err(storage_error)?;
                tracing::warn!(chunk_id=%chunk.id,speech_range_index=range_index,error_code=%error.code,"ai_processing_speech_range_failed");
                if first_failure.is_none() {
                    first_failure = Some(error);
                }
                continue;
            }
        };
        let windows = selection
            .audio_ranges
            .clone()
            .unwrap_or_else(|| vec![(0, input.slice_duration_ms)]);
        let owned = windows
            .iter()
            .map(|(start, end)| {
                (
                    (*start + prefix - range.start_ms) as f64 / 1000.0,
                    (*end + prefix - range.start_ms) as f64 / 1000.0,
                )
            })
            .collect::<Vec<_>>();
        let transcription = transcription::constrain_owned_transcription(
            transcription,
            range.end_ms - range.start_ms,
            &owned,
        );
        drop(range_audio.temp_audio);
        tracing::info!(
            chunk_id = %chunk.id,
            job_id = %chunk.job_id,
            speech_range_index = range_index,
            speech_range_start_ms = range.start_ms,
            speech_range_end_ms = range.end_ms,
            segment_count = transcription.segments.len(),
            word_count = transcription.words.len(),
            text_char_count = transcription
                .segments
                .iter()
                .map(|segment| segment.text.trim().chars().count())
                .sum::<usize>(),
            "AI transcription range completed."
        );
        check_atomic_cancel(&cancel)?;
        persist_speech_range(
            &storage,
            chunk,
            &input.track,
            &provider_name,
            &model,
            SpeechRangeResult {
                range: checkpoint,
                fingerprint: fingerprint.clone(),
                pending: transcript_persistence::PendingTranscription {
                    base_offset_ms: input.chunk_start_offset_ms + range.start_ms - prefix,
                    transcription,
                },
            },
        )?;
        storage
            .with_connection(|connection| {
                storage::update_processing_chunk_progress(
                    connection,
                    &chunk.id,
                    ((range_index + 1) as f64 / transcription_ranges.len() as f64).min(0.99),
                    now_ms(),
                )
                .map(|_| ())
            })
            .map_err(storage_error)?;
        emit_processing_state(app, &chunk.recording_id);
    }
    check_atomic_cancel(&cancel)?;
    ensure_chunk_active(&storage, chunk)?;
    if let Some(error) = first_failure {
        return Err(error);
    }
    if let Some(targets) = &selection.audio_ranges {
        storage
            .with_connection(|connection| {
                for (start, end) in targets {
                    storage::save_audio_range_checkpoint(
                        connection,
                        &chunk.id,
                        *start,
                        *end,
                        &fingerprint,
                        None,
                    )?;
                }
                Ok(())
            })
            .map_err(storage_error)?;
    }

    Ok(AudioChunkProcessingOutcome { result_code: None })
}
