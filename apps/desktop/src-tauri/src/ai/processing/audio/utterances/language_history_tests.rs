use super::*;
use tests::{fixture, stage, unit};

fn speech(language: &str, text: &str) -> transcription::VerboseTranscription {
    let mut result = crate::ai::tests::test_verbose_transcription(0., 1., text, vec![]);
    result.language = Some(language.into());
    result.language_confidence = Some(0.9);
    result.words = vec![transcription::VerboseTranscriptionWord {
        word: text.into(),
        start: 0.,
        end: 1.,
        probability: Some(0.9),
    }];
    result
}
#[test]
fn language_consensus_replacement_is_atomic_reversible_and_tracks_prior_source_privacy() {
    let directory = std::env::temp_dir().join(format!("daydream-language-{}", Uuid::now_v7()));
    std::fs::create_dir(&directory).unwrap();
    let database = directory.join("test.sqlite3");
    let storage = storage::test_file_storage(database.clone());
    let (chunk, track) = storage
        .with_connection(|connection| Ok(crate::ai::tests::test_transcription_chunk(connection)))
        .unwrap();
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
        publication::publish(
            &storage,
            &chunk,
            &track,
            &settings::load_app_settings(&storage).unwrap(),
            &unit,
            speech("en", "Synthetic original text"),
        )
        .unwrap();
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
    corrected.output_evidence = Some(state.clone());
    corrected.output_language = Some("de".into());
    let mut schedule = storage
        .with_connection(|connection| language_history::decision(connection, &current.id))
        .unwrap()
        .unwrap();
    schedule.repairs.retain(|id| id != &target.id);
    storage.with_connection(|connection| connection.execute_batch("CREATE TRIGGER fail_language_write BEFORE INSERT ON transcript_words BEGIN SELECT RAISE(ABORT,'synthetic failure'); END;")).unwrap();
    assert!(language_history::replace(
        &storage,
        &chunk,
        &track,
        &settings,
        language_history::Replacement {
            current,
            unit: &target,
            value: speech("de", "Synthetischer korrigierter Text."),
            language: &corrected,
            schedule: &schedule,
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
            assert!(language_history::decision(connection, &current.id)?
                .unwrap()
                .repairs
                .contains(&target.id));
            let old = rows.iter().find(|row| row.start_offset_ms == 0).unwrap();
            assert_eq!(
                storage::list_transcript_words(connection, &old.id)?[0].text,
                "Synthetic original text"
            );
            connection.execute_batch("DROP TRIGGER fail_language_write")
        })
        .unwrap();
    drop(storage);
    let storage = storage::test_file_storage(database.clone());
    storage
        .with_connection(|connection| {
            let evidence = corrected.output_evidence.as_ref().unwrap();
            connection.execute(
                "UPDATE processing_speech_units SET status='failed' WHERE id=?1",
                [&current.id],
            )?;
            assert!(language_history::output_supported(
                connection, &chunk, &track, &target, evidence
            )?);
            let next = unit(&chunk, &track, 4000, 5000);
            assert_eq!(
                language_history::previous(connection, &chunk, &track, &next)?
                    .confirmed
                    .as_deref(),
                Some("de")
            );
            let automatic = language_history::decision(connection, &current.id)?.unwrap();
            let mut forced = automatic.clone();
            forced.detected = Some("en".into());
            forced.output_language = Some("de".into());
            language_history::save(connection, &current.id, &forced)?;
            assert!(!language_history::output_supported(
                connection, &chunk, &track, &target, evidence
            )?);
            language_history::save(connection, &current.id, &automatic)?;
            connection.execute(
                "UPDATE processing_speech_units SET status='published' WHERE id=?1",
                [&current.id],
            )?;
            Ok(())
        })
        .unwrap();
    assert!(storage
        .with_connection(|connection| language_history::decision(connection, &current.id))
        .unwrap()
        .unwrap()
        .repairs
        .contains(&target.id));
    // Cancellation and redaction can occur while inference is outstanding.
    // Neither may commit the replacement or consume its durable queue entry.
    for blocked in [
        "paused",
        "cancelled",
        "redacted-target",
        "redacted-evidence",
    ] {
        let redaction = Uuid::now_v7().to_string();
        storage
            .with_connection(|connection| {
                if blocked.starts_with("redacted") {
                    let start = if blocked == "redacted-target" {
                        0
                    } else {
                        1000
                    };
                    storage::create_redaction(
                        connection,
                        &storage::CreateRedactionInput {
                            id: redaction.clone(),
                            recording_id: chunk.recording_id.clone(),
                            start_offset_ms: start,
                            end_offset_ms: start + 500,
                            track_ids_json: serde_json::to_string(&[&track.id]).unwrap(),
                            reason: None,
                            created_at_ms: 1,
                        },
                    )?;
                } else {
                    connection.execute(
                        "UPDATE processing_jobs SET status=?2 WHERE id=?1",
                        rusqlite::params![chunk.job_id, blocked],
                    )?;
                }
                Ok(())
            })
            .unwrap();
        assert!(
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
                    schedule: &schedule,
                }
            )
            .is_err(),
            "{blocked}"
        );
        storage
            .with_connection(|connection| {
                assert!(language_history::decision(connection, &current.id)?
                    .unwrap()
                    .repairs
                    .contains(&target.id));
                assert_eq!(
                    connection.query_row(
                        "SELECT text FROM transcript_segments WHERE start_offset_ms=0",
                        [],
                        |row| row.get::<_, String>(0)
                    )?,
                    "Synthetic original text"
                );
                if blocked.starts_with("redacted") {
                    storage::delete_redaction(connection, &chunk.recording_id, &redaction)?;
                } else {
                    connection.execute(
                        "UPDATE processing_jobs SET status='running' WHERE id=?1",
                        [&chunk.job_id],
                    )?;
                }
                Ok(())
            })
            .unwrap();
    }
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
            schedule: &schedule,
        },
    )
    .unwrap();
    drop(storage);
    let storage = storage::test_file_storage(database);
    let future = unit(&chunk, &track, 4000, 5000);
    storage
        .with_connection(|connection| {
            let repaired = language_history::decision(connection, &target.id)?.unwrap();
            assert_eq!(repaired.detected.as_deref(), Some("en"));
            assert_eq!(repaired.output_language.as_deref(), Some("de"));
            assert!(repaired.state.confirmed.is_none());
            assert_eq!(
                repaired
                    .output_evidence
                    .as_ref()
                    .unwrap()
                    .confirmed
                    .as_deref(),
                Some("de")
            );
            assert!(!language_history::decision(connection, &current.id)?
                .unwrap()
                .repairs
                .contains(&target.id));
            let early = unit(&chunk, &track, 1000, 2000);
            assert!(
                language_history::previous(connection, &chunk, &track, &early)?
                    .confirmed
                    .is_none()
            );
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
            let rows = storage::list_transcript_segments(connection, &chunk.recording_id)?;
            let replaced = rows.iter().find(|row| row.start_offset_ms == 0).unwrap();
            assert_eq!(replaced.chunk_id, target.owner_chunk_id);
            assert_eq!(replaced.segment_id, chunk.segment_id.clone().unwrap());
            let words = storage::list_transcript_words(connection, &replaced.id)?;
            assert_eq!(words.len(), 1);
            assert_eq!(words[0].text, "Synthetischer korrigierter Text.");
            assert_eq!(
                (words[0].start_offset_ms, words[0].end_offset_ms),
                (0, 1000)
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
    drop(storage);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn repair_candidates_keep_history_job_track_recording_and_configuration_boundaries() {
    let (storage, chunk, track) = fixture();
    let earlier = unit(&chunk, &track, 0, 1000);
    stage(&storage, &earlier);
    let earlier = storage
        .with_connection(|connection| storage::speech_unit(connection, &earlier.id))
        .unwrap()
        .unwrap();
    storage
        .with_connection(|connection| {
            let mut current = unit(
                &chunk,
                &track,
                language::HISTORY_MS,
                language::HISTORY_MS + 1000,
            );
            assert!(language_history::eligible(
                connection, &chunk, &track, &current, &earlier
            )?);
            current.start_ms += 1;
            assert!(!language_history::eligible(
                connection, &chunk, &track, &current, &earlier
            )?);
            current.start_ms = 1000;
            for change in ["job", "track", "config", "status", "overlap"] {
                let mut incompatible = earlier.clone();
                match change {
                    "job" => incompatible.job_id = "another-job".into(),
                    "track" => incompatible.track_id = "desktop-audio".into(),
                    "config" => incompatible.fingerprint = "another-config".into(),
                    "status" => incompatible.status = "queued".into(),
                    _ => incompatible.end_ms = 1001,
                }
                assert!(
                    !language_history::eligible(
                        connection,
                        &chunk,
                        &track,
                        &current,
                        &incompatible
                    )?,
                    "{change}"
                );
            }
            let mut another_recording = chunk.clone();
            another_recording.recording_id = "another-recording".into();
            assert!(!language_history::eligible(
                connection,
                &another_recording,
                &track,
                &current,
                &earlier
            )?);
            Ok(())
        })
        .unwrap();
}

#[test]
fn saved_decisions_without_correction_provenance_remain_compatible() {
    let original = language::plan(
        language::State::default(),
        "ack",
        0,
        1000,
        &speech("en", "Yes."),
    );
    let mut json = serde_json::to_value(&original).unwrap();
    json.as_object_mut().unwrap().remove("output_evidence");
    let restored: language::Decision = serde_json::from_value(json).unwrap();
    assert_eq!(restored.state, original.state);
    assert!(restored.output_evidence.is_none());
    let response = speech("en", "Synthetic original speech.");
    let old_json = serde_json::to_string(&response).unwrap();
    let checkpoint: language_history::Checkpoint<transcription::VerboseTranscription> =
        serde_json::from_str(&old_json).unwrap();
    assert!(!checkpoint.language_corrected);
    assert_eq!(checkpoint.value.text, response.text);
}

#[test]
fn staged_correction_remains_marked_when_its_decision_evidence_is_deleted() {
    let (storage, chunk, track) = fixture();
    storage
        .with_connection(|connection| {
            connection.execute(
                "UPDATE recording_segments SET duration_ms=5000,state='finalized' WHERE id=?1",
                [chunk.segment_id.as_ref().unwrap()],
            )?;
            storage::mark_recording_completed(connection, &chunk.recording_id, 5001)
        })
        .unwrap();
    let mut state = language::State::default();
    let mut units = Vec::new();
    for number in 0..4 {
        let unit = unit(&chunk, &track, number * 1000, number * 1000 + 1000);
        stage(&storage, &unit);
        let sample = if number == 0 {
            speech("en", "Yes.")
        } else {
            speech("de", "Dies ist synthetische Sprache.")
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
    let mut schedule = storage
        .with_connection(|connection| language_history::decision(connection, &current.id))
        .unwrap()
        .unwrap();
    schedule.repairs.retain(|id| id != &target.id);
    let mut corrected = storage
        .with_connection(|connection| language_history::decision(connection, &target.id))
        .unwrap()
        .unwrap();
    corrected.output_language = Some("de".into());
    corrected.output_evidence = Some(state);
    language_history::replace(
        &storage,
        &chunk,
        &track,
        &settings::load_app_settings(&storage).unwrap(),
        language_history::Replacement {
            current,
            unit: &target,
            value: speech("de", "Synthetische Korrektur."),
            language: &corrected,
            schedule: &schedule,
        },
    )
    .unwrap();
    storage
        .with_connection(|connection| {
            connection.execute(
                "DELETE FROM processing_speech_units WHERE id=?1",
                [&units[1].id],
            )?;
            assert!(language_history::decision(connection, &target.id)?.is_none());
            let saved = storage::speech_unit(connection, &target.id)?.unwrap();
            let checkpoint: language_history::Checkpoint<transcription::VerboseTranscription> =
                serde_json::from_str(saved.output_json.as_ref().unwrap()).unwrap();
            assert!(checkpoint.language_corrected);
            assert_eq!(checkpoint.value.text, "Synthetische Korrektur.");
            Ok(())
        })
        .unwrap();
}
