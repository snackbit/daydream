use super::*;
use tests::{fixture, publish, stage, unit};

fn speech(language: &str, text: &str) -> transcription::VerboseTranscription {
    let mut result = crate::ai::tests::test_verbose_transcription(0., 1., text, vec![]);
    result.language = Some(language.into());
    result.language_confidence = Some(0.9);
    result
}
#[test]
fn language_consensus_replacement_is_atomic_reversible_and_tracks_prior_source_privacy() {
    let (storage, chunk, track) = fixture();
    storage
        .with_connection(|connection| {
            storage::finalize_recording_segment(
                connection,
                &storage::FinalizeRecordingSegmentInput {
                    id: chunk.segment_id.clone().unwrap(),
                    duration_ms: 5000,
                    ended_at_ms: 5001,
                    updated_at_ms: 5001,
                    video_stream_index: None,
                    desktop_audio_stream_index: None,
                    microphone_audio_stream_index: Some(2),
                    probe_json: None,
                },
            )?;
            storage::mark_recording_completed(connection, &chunk.recording_id, 5001)
        })
        .unwrap();
    let mut state = language::State::default();
    let mut units = Vec::new();
    for number in 0..4 {
        let unit = unit(&chunk, &track, number * 1000, number * 1000 + 1000);
        stage(&storage, &unit);
        publish(&storage, &chunk, &track, &unit, "Synthetic original text").unwrap();
        let sample = if number == 0 {
            speech("en", "Mm-hmm.")
        } else {
            speech("de", "Dies ist ein synthetischer Beispielsatz.")
        };
        let decision = language::plan(state, &unit.id, unit.start_ms, unit.end_ms, &sample);
        storage
            .with_connection(|connection| language_history::save(connection, &unit.id, &decision))
            .unwrap();
        state = decision.state;
        units.push(unit);
    }
    let target = storage
        .with_connection(|connection| storage::speech_unit(connection, &units[0].id))
        .unwrap()
        .unwrap();
    let current = &units[3];
    let settings = settings::load_app_settings(&storage).unwrap();
    let mut corrected = storage
        .with_connection(|connection| language_history::decision(connection, &target.id))
        .unwrap()
        .unwrap();
    corrected.state = state.clone();
    corrected.output_language = Some("de".into());
    storage.with_connection(|connection| connection.execute_batch("CREATE TRIGGER fail_language_write BEFORE INSERT ON transcript_segments BEGIN SELECT RAISE(ABORT,'synthetic failure'); END;")).unwrap();
    assert!(language_history::replace(
        &storage,
        &chunk,
        &track,
        &settings,
        language_history::Replacement {
            current,
            unit: &target,
            value: speech("de", "Synthetischer korrigierter Text."),
            language: &corrected
        },
    )
    .is_err());
    storage
        .with_connection(|connection| {
            let rows = storage::list_transcript_segments(connection, &chunk.recording_id)?;
            assert_eq!(
                rows.iter().filter(|row| row.start_offset_ms == 0).count(),
                1
            );
            assert_eq!(
                rows.iter()
                    .find(|row| row.start_offset_ms == 0)
                    .unwrap()
                    .text,
                "Synthetic original text"
            );
            connection.execute_batch("DROP TRIGGER fail_language_write")
        })
        .unwrap();
    language_history::replace(
        &storage,
        &chunk,
        &track,
        &settings,
        language_history::Replacement {
            current,
            unit: &target,
            value: speech("de", "Synthetischer korrigierter Text."),
            language: &corrected,
        },
    )
    .unwrap();
    let future = unit(&chunk, &track, 4000, 5000);
    storage
        .with_connection(|connection| {
            assert_eq!(
                language_history::previous(connection, &chunk, &track, &future)?
                    .confirmed
                    .as_deref(),
                Some("de")
            );
            assert_eq!(
                storage::list_transcript_segments(connection, &chunk.recording_id)?
                    .iter()
                    .find(|row| row.start_offset_ms == 0)
                    .unwrap()
                    .text,
                "Synthetischer korrigierter Text."
            );
            let cut = |id: &str, targets: &str| {
                storage::create_redaction(
                    connection,
                    &storage::CreateRedactionInput {
                        id: id.into(),
                        recording_id: chunk.recording_id.clone(),
                        start_offset_ms: 1000,
                        end_offset_ms: 1500,
                        track_ids_json: targets.into(),
                        reason: None,
                        created_at_ms: 1,
                    },
                )
            };
            let unrelated = Uuid::now_v7().to_string();
            cut(&unrelated, "[\"desktop-audio-1\"]")?;
            assert!(
                storage::list_transcript_segments(connection, &chunk.recording_id)?
                    .iter()
                    .any(|row| row.start_offset_ms == 0)
            );
            storage::delete_redaction(connection, &chunk.recording_id, &unrelated)?;
            let hidden = Uuid::now_v7().to_string();
            cut(&hidden, &serde_json::to_string(&[&track.id]).unwrap())?;
            assert!(
                !storage::list_transcript_segments(connection, &chunk.recording_id)?
                    .iter()
                    .any(|row| row.start_offset_ms == 0)
            );
            assert!(
                language_history::previous(connection, &chunk, &track, &future)?
                    .confirmed
                    .is_none()
            );
            storage::delete_redaction(connection, &chunk.recording_id, &hidden)?;
            assert!(
                storage::list_transcript_segments(connection, &chunk.recording_id)?
                    .iter()
                    .any(|row| row.start_offset_ms == 0)
            );
            connection.execute(
                "DELETE FROM processing_speech_units WHERE id=?1",
                [&units[1].id],
            )?;
            assert!(
                !storage::list_transcript_segments(connection, &chunk.recording_id)?
                    .iter()
                    .any(|row| row.start_offset_ms == 0)
            );
            connection.execute("DELETE FROM recordings WHERE id=?1", [&chunk.recording_id])?;
            assert_eq!(
                connection.query_row(
                    "SELECT COUNT(*) FROM processing_speech_language_support",
                    [],
                    |row| row.get::<_, i64>(0)
                )?,
                0
            );
            assert_eq!(
                connection.query_row(
                    "SELECT COUNT(*) FROM processing_speech_languages",
                    [],
                    |row| row.get::<_, i64>(0)
                )?,
                0
            );
            Ok(())
        })
        .unwrap();
}
