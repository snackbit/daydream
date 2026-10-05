use super::super::*;

mod chunk_input;
mod extraction;
mod ranges;
mod transcript_persistence;

use chunk_input::load_audio_chunk_input;
use extraction::extract_audio_stream;
use ranges::{audio_path_for_speech_range, plan_transcription_ranges};
use transcript_persistence::persist_transcriptions;

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
    let app_settings = settings::load_app_settings(&storage).map_err(settings_error)?;
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
    extract_audio_stream(
        &input.segment.file_path,
        input.stream_index,
        input.slice_start_ms,
        input.slice_duration_ms,
        temp_audio.path(),
        &cancel,
    )?;
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

    let model = selected_transcription_model(&app_settings)?;
    let provider = app.state::<TranscriptionProviderManager>();
    let transcription_ranges = plan_transcription_ranges(
        temp_audio.path(),
        input.slice_duration_ms,
        app_settings.ai.transcription.silence_gate_enabled,
        i64::from(app_settings.ai.transcription.merge_gap_ms),
        i64::from(app_settings.ai.transcription.min_speech_ms),
        app_settings.ai.transcription.silence_threshold_db,
    )?;

    storage
        .with_connection(|connection| {
            storage::delete_transcript_output_for_chunks(
                connection,
                std::slice::from_ref(&chunk.id),
            )
        })
        .map_err(storage_error)?;

    if transcription_ranges.is_empty() {
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

    tracing::info!(
        chunk_id = %chunk.id,
        job_id = %chunk.job_id,
        model = %model,
        provider = %provider_name,
        backend = %app_settings.ai.transcription.local_backend,
        speech_range_count = transcription_ranges.len(),
        "AI transcription started."
    );
    let mut pending_transcriptions = Vec::with_capacity(transcription_ranges.len());
    for (range_index, range) in transcription_ranges.iter().enumerate() {
        check_atomic_cancel(&cancel)?;
        ensure_chunk_active(&storage, chunk)?;
        let range_audio = audio_path_for_speech_range(
            app,
            chunk,
            &input,
            range,
            range_index,
            temp_audio.path(),
            &cancel,
        )?;
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
            )
            .await
            .map_err(transcription_error)?;
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
        pending_transcriptions.push(transcript_persistence::PendingTranscription {
            base_offset_ms: input.chunk_start_offset_ms + range.start_ms,
            transcription,
        });
    }
    check_atomic_cancel(&cancel)?;
    ensure_chunk_active(&storage, chunk)?;
    persist_transcriptions(
        app,
        chunk,
        &input.track,
        &provider_name,
        &model,
        pending_transcriptions,
    )?;

    Ok(AudioChunkProcessingOutcome { result_code: None })
}
