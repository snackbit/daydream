use super::*;

pub(in crate::ai) struct PendingTranscription {
    pub(in crate::ai) base_offset_ms: i64,
    pub(in crate::ai) transcription: transcription::VerboseTranscription,
}

#[derive(Debug, Default)]
pub(in crate::ai) struct PersistedTranscriptionStats {
    pub(in crate::ai) segment_count: usize,
    pub(in crate::ai) text_char_count: usize,
    pub(in crate::ai) word_count: usize,
}

pub(super) fn persist_transcriptions(
    app: &AppHandle,
    chunk: &ProcessingChunkRow,
    track: &RecordingTrackRow,
    provider: &str,
    model: &str,
    transcriptions: Vec<PendingTranscription>,
) -> Result<(), AiError> {
    if transcriptions
        .iter()
        .all(|pending| pending.transcription.segments.is_empty())
    {
        tracing::info!(
            recording_id = %chunk.recording_id,
            job_id = %chunk.job_id,
            chunk_id = %chunk.id,
            track_id = chunk.track_id.as_deref().unwrap_or(""),
            source_role = %track.kind,
            "ai_processing_transcript_persist_skipped_empty"
        );
        return Ok(());
    }
    let segment_id = chunk.segment_id.clone().ok_or_else(|| {
        ai_error(
            "ai_processing_segment_missing",
            "Processing segment is missing.",
        )
    })?;
    let storage = app.state::<AppStorage>();
    let stats = storage
        .with_connection_mut(|connection| {
            persist_transcriptions_for_chunk(
                connection,
                chunk,
                track,
                provider,
                model,
                &segment_id,
                transcriptions,
            )
        })
        .map_err(storage_error)?;
    if stats.segment_count == 0 {
        tracing::info!(
            recording_id = %chunk.recording_id,
            job_id = %chunk.job_id,
            chunk_id = %chunk.id,
            track_id = chunk.track_id.as_deref().unwrap_or(""),
            source_role = %track.kind,
            "ai_processing_transcript_persist_skipped_empty"
        );
        return Ok(());
    }
    tracing::info!(
        recording_id = %chunk.recording_id,
        job_id = %chunk.job_id,
        chunk_id = %chunk.id,
        track_id = %track.id,
        source_role = %track.kind,
        segment_count = stats.segment_count,
        word_count = stats.word_count,
        text_char_count = stats.text_char_count,
        "ai_processing_transcript_persisted"
    );
    Ok(())
}

pub(in crate::ai) fn persist_transcriptions_for_chunk(
    connection: &mut rusqlite::Connection,
    chunk: &ProcessingChunkRow,
    track: &RecordingTrackRow,
    provider: &str,
    model: &str,
    segment_id: &str,
    transcriptions: Vec<PendingTranscription>,
) -> rusqlite::Result<PersistedTranscriptionStats> {
    let transaction = connection.transaction()?;
    let mut stats = PersistedTranscriptionStats::default();
    if !crate::recording_privacy::chunk_active(&transaction, chunk)? {
        return Ok(stats);
    }

    for pending in transcriptions {
        for segment in pending.transcription.segments {
            let text = segment.text.trim().to_string();
            if text.is_empty() {
                continue;
            }
            let transcript_segment_id = Uuid::now_v7().to_string();
            let segment_words =
                transcription_words_for_segment(&segment, &pending.transcription.words);
            let (start_offset_ms, end_offset_ms) =
                transcript_segment_offsets_ms(pending.base_offset_ms, &segment, &segment_words);
            storage::create_transcript_segment(
                &transaction,
                &CreateTranscriptSegmentInput {
                    chunk_id: chunk.id.clone(),
                    confidence: None,
                    created_at_ms: now_ms(),
                    end_offset_ms,
                    id: transcript_segment_id.clone(),
                    job_id: chunk.job_id.clone(),
                    model: Some(model.to_string()),
                    provider: provider.to_string(),
                    recording_id: chunk.recording_id.clone(),
                    segment_id: segment_id.to_string(),
                    source_role: track.kind.clone(),
                    start_offset_ms,
                    text: text.clone(),
                    track_id: track.id.clone(),
                },
            )?;
            stats.segment_count += 1;
            stats.text_char_count += text.chars().count();
            for word in segment_words {
                storage::create_transcript_word(
                    &transaction,
                    &CreateTranscriptWordInput {
                        end_offset_ms: pending.base_offset_ms + seconds_to_ms(word.end),
                        id: Uuid::now_v7().to_string(),
                        probability: word.probability,
                        start_offset_ms: pending.base_offset_ms + seconds_to_ms(word.start),
                        text: word.word.trim().to_string(),
                        transcript_segment_id: transcript_segment_id.clone(),
                    },
                )?;
                stats.word_count += 1;
            }
        }
    }

    transaction.commit()?;
    Ok(stats)
}

pub(in crate::ai) fn transcription_words_for_segment<'a>(
    segment: &transcription::VerboseTranscriptionSegment,
    words: &'a [transcription::VerboseTranscriptionWord],
) -> Vec<&'a transcription::VerboseTranscriptionWord> {
    words
        .iter()
        .filter(|word| {
            word.start >= segment.start
                && word.end <= segment.end
                && word.end > word.start
                && word.start.is_finite()
                && word.end.is_finite()
                && !word.word.trim().is_empty()
        })
        .collect()
}

pub(in crate::ai) fn transcript_segment_offsets_ms(
    base_offset_ms: i64,
    segment: &transcription::VerboseTranscriptionSegment,
    words: &[&transcription::VerboseTranscriptionWord],
) -> (i64, i64) {
    let start_seconds = words
        .iter()
        .map(|word| word.start)
        .reduce(f64::min)
        .unwrap_or(segment.start);
    let end_seconds = words
        .iter()
        .map(|word| word.end)
        .reduce(f64::max)
        .unwrap_or(segment.end);
    let start_offset_ms = base_offset_ms + seconds_to_ms(start_seconds);
    let end_offset_ms = base_offset_ms + seconds_to_ms(end_seconds);

    (start_offset_ms, end_offset_ms.max(start_offset_ms))
}
