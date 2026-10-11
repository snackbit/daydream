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

pub(super) struct SpeechRangeResult {
    pub(super) language_context: Option<storage::TranscriptLanguageContext>,
    pub(super) range: whisper_backend::SpeechRange,
    pub(super) fingerprint: String,
    pub(super) pending: PendingTranscription,
}

struct TranscriptionBatch<'a> {
    transcriptions: Vec<PendingTranscription>,
    language: Option<&'a storage::TranscriptLanguageContext>,
    unit_id: Option<&'a str>,
}

pub(super) fn persist_speech_range(
    storage: &AppStorage,
    chunk: &ProcessingChunkRow,
    track: &RecordingTrackRow,
    provider: &str,
    model: &str,
    result: SpeechRangeResult,
) -> Result<(), AiError> {
    let saved = storage
        .with_connection_mut(|connection| {
            let transaction = connection.transaction()?;
            if !crate::recording_privacy::chunk_active(&transaction, chunk)? {
                return Ok(false);
            }
            if storage::audio_range_succeeded(
                &transaction,
                &chunk.id,
                result.range.start_ms,
                result.range.end_ms,
                &result.fingerprint,
            )? {
                return Ok(true);
            }
            let segment_id = chunk
                .segment_id
                .as_deref()
                .ok_or(rusqlite::Error::InvalidQuery)?;
            write_transcriptions(
                &transaction,
                chunk,
                track,
                provider,
                model,
                segment_id,
                TranscriptionBatch {
                    transcriptions: vec![result.pending],
                    language: result.language_context.as_ref(),
                    unit_id: None,
                },
            )?;
            storage::save_audio_range_checkpoint(
                &transaction,
                &chunk.id,
                result.range.start_ms,
                result.range.end_ms,
                &result.fingerprint,
                None,
            )?;
            transaction.commit()?;
            Ok(true)
        })
        .map_err(storage_error)?;
    if !saved {
        return Err(ai_error(
            "ai_processing_cancelled",
            "The audio source is no longer eligible.",
        ));
    }
    Ok(())
}

#[cfg(test)]
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
    if !crate::recording_privacy::chunk_active(&transaction, chunk)? {
        return Ok(PersistedTranscriptionStats::default());
    }
    let stats = write_transcriptions(
        &transaction,
        chunk,
        track,
        provider,
        model,
        segment_id,
        TranscriptionBatch {
            transcriptions,
            language: None,
            unit_id: None,
        },
    )?;
    transaction.commit()?;
    Ok(stats)
}

pub(super) fn write_speech_unit(
    connection: &rusqlite::Connection,
    chunk: &ProcessingChunkRow,
    track: &RecordingTrackRow,
    settings: &settings::AppSettings,
    model: &str,
    unit: &storage::SpeechUnit,
    value: transcription::VerboseTranscription,
) -> rusqlite::Result<()> {
    let segment_id = chunk
        .segment_id
        .as_deref()
        .ok_or(rusqlite::Error::InvalidQuery)?;
    let language = language_context::result_context(
        &settings.ai.transcription.language,
        storage::preceding_transcript_language(
            connection,
            chunk,
            &track.id,
            unit.start_ms,
            &unit.fingerprint,
            &settings.ai.transcription.provider,
            model,
        )?,
        &value,
        &unit.fingerprint,
        unit.submitted_start_ms,
        unit.submitted_end_ms,
    );
    write_transcriptions(
        connection,
        chunk,
        track,
        &settings.ai.transcription.provider,
        model,
        segment_id,
        TranscriptionBatch {
            transcriptions: vec![PendingTranscription {
                base_offset_ms: unit.submitted_start_ms,
                transcription: value,
            }],
            language: language.as_ref(),
            unit_id: Some(&unit.id),
        },
    )?;
    Ok(())
}

fn write_transcriptions(
    connection: &rusqlite::Connection,
    chunk: &ProcessingChunkRow,
    track: &RecordingTrackRow,
    provider: &str,
    model: &str,
    segment_id: &str,
    batch: TranscriptionBatch<'_>,
) -> rusqlite::Result<PersistedTranscriptionStats> {
    let TranscriptionBatch {
        transcriptions,
        language,
        unit_id,
    } = batch;
    let mut stats = PersistedTranscriptionStats::default();
    let sources = storage::list_recording_segments(connection, &chunk.recording_id)?;
    let bases = storage::segment_playback_bases(&sources);
    let mut detection_id = language.and_then(|context| context.detection_segment_id.clone());
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
                connection,
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
                    segment_id: sources
                        .iter()
                        .zip(&bases)
                        .find(|(source, base)| {
                            start_offset_ms >= **base
                                && start_offset_ms < **base + source.duration_ms.unwrap_or(0)
                        })
                        .map(|(source, _)| source.id.as_str())
                        .unwrap_or(segment_id)
                        .to_string(),
                    source_role: track.kind.clone(),
                    start_offset_ms,
                    text: text.clone(),
                    track_id: track.id.clone(),
                },
            )?;
            if let Some(unit_id) = unit_id {
                connection.execute("INSERT INTO processing_speech_unit_transcripts(unit_id,transcript_id) VALUES(?1,?2)",rusqlite::params![unit_id,transcript_segment_id])?;
            }
            if let Some(context) = language {
                let id = detection_id.get_or_insert_with(|| transcript_segment_id.clone());
                storage::save_transcript_language_context(
                    connection,
                    &transcript_segment_id,
                    id,
                    context,
                )?;
            }
            stats.segment_count += 1;
            stats.text_char_count += text.chars().count();
            for word in segment_words {
                storage::create_transcript_word(
                    connection,
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

#[cfg(test)]
mod checkpoint_tests {
    use super::*;
    #[test]
    fn committed_speech_is_idempotent_and_a_failed_checkpoint_rolls_back_text() {
        let storage = storage::test_storage(PathBuf::new());
        let (chunk, track) = storage
            .with_connection(|connection| {
                let (chunk, track) = crate::ai::tests::test_transcription_chunk(connection);
                storage::finalize_recording_segment(
                    connection,
                    &storage::FinalizeRecordingSegmentInput {
                        id: chunk.segment_id.clone().unwrap(),
                        duration_ms: 5000,
                        ended_at_ms: 5001,
                        updated_at_ms: 5001,
                        video_stream_index: Some(0),
                        desktop_audio_stream_index: Some(1),
                        microphone_audio_stream_index: Some(2),
                        probe_json: None,
                    },
                )?;
                Ok((chunk, track))
            })
            .unwrap();
        let result = || SpeechRangeResult {
            language_context: None,
            range: whisper_backend::SpeechRange {
                start_ms: 100,
                end_ms: 600,
            },
            fingerprint: "synthetic-config".into(),
            pending: PendingTranscription {
                base_offset_ms: 100,
                transcription: crate::ai::tests::test_verbose_transcription(
                    0.0,
                    0.5,
                    "Synthetic reply",
                    vec![],
                ),
            },
        };
        persist_speech_range(
            &storage,
            &chunk,
            &track,
            "local-whisper-cpp",
            "synthetic",
            result(),
        )
        .unwrap();
        persist_speech_range(
            &storage,
            &chunk,
            &track,
            "local-whisper-cpp",
            "synthetic",
            result(),
        )
        .unwrap();
        storage.with_connection(|connection|{
            assert_eq!(connection.query_row("SELECT COUNT(*) FROM transcript_segments WHERE chunk_id=?1",[&chunk.id],|row|row.get::<_,i64>(0))?,1);
            connection.execute_batch("CREATE TRIGGER reject_speech_checkpoint BEFORE INSERT ON processing_audio_ranges BEGIN SELECT RAISE(ABORT,'synthetic failure'); END;")?;Ok(())
        }).unwrap();
        let mut failed = result();
        failed.range.start_ms = 1000;
        failed.range.end_ms = 1500;
        failed.pending.base_offset_ms = 1000;
        assert!(persist_speech_range(
            &storage,
            &chunk,
            &track,
            "local-whisper-cpp",
            "synthetic",
            failed
        )
        .is_err());
        storage
            .with_connection(|connection| {
                assert_eq!(
                    connection.query_row(
                        "SELECT COUNT(*) FROM transcript_segments WHERE chunk_id=?1",
                        [&chunk.id],
                        |row| row.get::<_, i64>(0)
                    )?,
                    1
                );
                Ok(())
            })
            .unwrap();
    }
}
