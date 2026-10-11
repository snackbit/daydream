use super::*;
use crate::ai::processing::audio::transcript_persistence::{
    persist_speech_range, PendingTranscription, SpeechRangeResult,
};

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let path =
            std::env::temp_dir().join(format!("daydream-language-context-{}", Uuid::now_v7()));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn fixture(storage: &AppStorage) -> (ProcessingChunkRow, RecordingTrackRow) {
    storage
        .with_connection(|connection| {
            let (mut chunk, track) = crate::ai::tests::test_transcription_chunk(connection);
            storage::finalize_recording_segment(
                connection,
                &storage::FinalizeRecordingSegmentInput {
                    id: chunk.segment_id.clone().unwrap(),
                    duration_ms: 100_000,
                    ended_at_ms: 100_001,
                    updated_at_ms: 100_001,
                    video_stream_index: None,
                    desktop_audio_stream_index: Some(1),
                    microphone_audio_stream_index: Some(2),
                    probe_json: None,
                },
            )?;
            connection.execute(
                "UPDATE processing_chunks SET end_offset_ms=100000 WHERE id=?1",
                [&chunk.id],
            )?;
            chunk.end_offset_ms = Some(100_000);
            Ok((chunk, track))
        })
        .unwrap()
}

fn save(
    storage: &AppStorage,
    chunk: &ProcessingChunkRow,
    track: &RecordingTrackRow,
    start: i64,
    previous: Option<storage::TranscriptLanguageContext>,
) {
    let mut transcription =
        crate::ai::tests::test_verbose_transcription(0.0, 1.0, "Synthetic phrase", vec![]);
    transcription.language = Some("de".into());
    transcription.language_confidence = previous.is_none().then_some(0.9);
    let context = result_context(
        "auto",
        previous,
        &transcription,
        "config",
        start,
        start + 1000,
    );
    let local_start = start - chunk.start_offset_ms.unwrap_or(0);
    persist_speech_range(
        storage,
        chunk,
        track,
        "local-whisper-cpp",
        "synthetic",
        SpeechRangeResult {
            language_context: context,
            range: whisper_backend::SpeechRange {
                start_ms: local_start,
                end_ms: local_start + 1000,
            },
            fingerprint: "config".into(),
            pending: PendingTranscription {
                base_offset_ms: start,
                transcription,
            },
        },
    )
    .unwrap();
}

fn read(
    storage: &AppStorage,
    chunk: &ProcessingChunkRow,
    track: &RecordingTrackRow,
    start: i64,
) -> Option<storage::TranscriptLanguageContext> {
    storage
        .with_connection(|connection| {
            storage::preceding_transcript_language(
                connection,
                chunk,
                &track.id,
                start,
                "config",
                "local-whisper-cpp",
                "synthetic",
            )
        })
        .unwrap()
}

#[test]
fn language_context_migration_preserves_legacy_transcripts_without_inventing_confidence() {
    let directory = TestDirectory::new();
    let path = directory.0.join("legacy.sqlite3");
    let mut connection = rusqlite::Connection::open(&path).unwrap();
    storage::migrations()
        .to_version(&mut connection, 30)
        .unwrap();
    connection.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
    let (chunk, track) = crate::ai::tests::test_transcription_chunk(&connection);
    let id = Uuid::now_v7().to_string();
    storage::create_transcript_segment(
        &connection,
        &storage::CreateTranscriptSegmentInput {
            id: id.clone(),
            chunk_id: chunk.id.clone(),
            job_id: chunk.job_id.clone(),
            recording_id: chunk.recording_id.clone(),
            segment_id: chunk.segment_id.clone().unwrap(),
            track_id: track.id.clone(),
            source_role: track.kind.clone(),
            start_offset_ms: 0,
            end_offset_ms: 1000,
            text: "Synthetic legacy phrase".into(),
            provider: "local-whisper-cpp".into(),
            model: Some("synthetic".into()),
            confidence: None,
            created_at_ms: 1,
        },
    )
    .unwrap();
    drop(connection);
    let storage = storage::test_file_storage(path);
    storage
        .with_connection(|connection| {
            assert_eq!(
                storage::get_transcript_segment(connection, &id)?
                    .unwrap()
                    .text,
                "Synthetic legacy phrase"
            );
            assert_eq!(
                connection.query_row(
                    "SELECT COUNT(*) FROM transcript_language_context",
                    [],
                    |row| row.get::<_, i64>(0)
                )?,
                0
            );
            Ok(())
        })
        .unwrap();
    assert!(read(&storage, &chunk, &track, 2000).is_none());
}

#[test]
fn language_context_renews_on_speech_expires_after_fifteen_seconds_and_survives_restart() {
    let directory = TestDirectory::new();
    let path = directory.0.join("synthetic.sqlite3");
    let storage = storage::test_file_storage(path.clone());
    let (chunk, track) = fixture(&storage);
    save(&storage, &chunk, &track, 0, None);
    assert!(read(&storage, &chunk, &track, 16000).is_some());
    assert!(read(&storage, &chunk, &track, 16001).is_none());
    assert!(read(&storage, &chunk, &track, 0).is_none());
    let previous = read(&storage, &chunk, &track, 12000).unwrap();
    save(&storage, &chunk, &track, 12000, Some(previous));
    drop(storage);
    let storage = storage::test_file_storage(path);
    let previous = read(&storage, &chunk, &track, 26000).unwrap();
    assert_eq!(previous.language, "de");
    assert_eq!(previous.detection_start_ms, 0);
    save(&storage, &chunk, &track, 26000, Some(previous));
    assert!(read(&storage, &chunk, &track, 42000).is_some());
    assert!(read(&storage, &chunk, &track, 42001).is_none());
    storage
        .with_connection(|connection| {
            assert!(storage::preceding_transcript_language(
                connection,
                &chunk,
                &track.id,
                28000,
                "other-config",
                "local-whisper-cpp",
                "synthetic"
            )?
            .is_none());
            assert!(storage::preceding_transcript_language(
                connection,
                &chunk,
                &track.id,
                28000,
                "config",
                "local-whisper-cpp",
                "other-model"
            )?
            .is_none());
            let mut other = chunk.clone();
            other.recording_id = "other-recording".into();
            assert!(storage::preceding_transcript_language(
                connection,
                &other,
                &track.id,
                28000,
                "config",
                "local-whisper-cpp",
                "synthetic"
            )?
            .is_none());
            Ok(())
        })
        .unwrap();
}

#[test]
fn language_context_excludes_redacted_detections_even_when_recent_speech_is_visible() {
    for target in ["uuid", "microphone-2", "all"] {
        let storage = storage::test_storage(PathBuf::new());
        let (chunk, track) = fixture(&storage);
        save(&storage, &chunk, &track, 0, None);
        let context = read(&storage, &chunk, &track, 12000).unwrap();
        save(&storage, &chunk, &track, 12000, Some(context));
        storage
            .with_connection(|connection| {
                storage::create_redaction(
                    connection,
                    &storage::CreateRedactionInput {
                        id: Uuid::now_v7().to_string(),
                        recording_id: chunk.recording_id.clone(),
                        start_offset_ms: 0,
                        end_offset_ms: 1000,
                        created_at_ms: 1,
                        track_ids_json: match target {
                            "uuid" => serde_json::to_string(&[&track.id]).unwrap(),
                            "all" => "[]".into(),
                            _ => serde_json::to_string(&[target]).unwrap(),
                        },
                        reason: None,
                    },
                )?;
                Ok(())
            })
            .unwrap();
        assert!(
            read(&storage, &chunk, &track, 14000).is_none(),
            "target {target}"
        );
        storage
            .with_connection(|connection| {
                connection.execute("DELETE FROM redactions", [])?;
                Ok(())
            })
            .unwrap();
        assert!(read(&storage, &chunk, &track, 14000).is_some());
        storage
            .with_connection(|connection| {
                connection.execute(
                    "DELETE FROM transcript_segments WHERE start_offset_ms=0",
                    [],
                )?;
                assert_eq!(
                    connection.query_row(
                        "SELECT COUNT(*) FROM transcript_language_context",
                        [],
                        |row| row.get::<_, i64>(0)
                    )?,
                    0
                );
                Ok(())
            })
            .unwrap();
        assert!(read(&storage, &chunk, &track, 14000).is_none());
    }
}

#[test]
fn language_context_is_track_local_and_rolls_back_with_failed_publication() {
    let storage = storage::test_storage(PathBuf::new());
    let (chunk, track) = fixture(&storage);
    save(&storage, &chunk, &track, 0, None);
    storage.with_connection(|connection| {
        let mut input = storage::UpsertRecordingTrackInput { id:Uuid::now_v7().to_string(),recording_id:chunk.recording_id.clone(),kind:"desktop-audio".into(),label:"Desktop audio".into(),created_at_ms:1,codec:None,path:None,source_id:None };
        storage::upsert_recording_track(connection, &input)?;
        assert!(storage::preceding_transcript_language(connection, &chunk, &input.id, 2000, "config", "local-whisper-cpp", "synthetic")?.is_none());
        input.id = Uuid::now_v7().to_string(); input.kind = "microphone".into();
        storage::upsert_recording_track(connection, &input)?;
        assert!(storage::preceding_transcript_language(connection, &chunk, &input.id, 2000, "config", "local-whisper-cpp", "synthetic")?.is_none());
        connection.execute_batch("CREATE TRIGGER reject_language_checkpoint BEFORE INSERT ON processing_audio_ranges BEGIN SELECT RAISE(ABORT,'synthetic failure'); END;")?;
        Ok(())
    }).unwrap();
    let previous = read(&storage, &chunk, &track, 12000).unwrap();
    let transcription =
        crate::ai::tests::test_verbose_transcription(0.0, 1.0, "Synthetic phrase", vec![]);
    assert!(persist_speech_range(
        &storage,
        &chunk,
        &track,
        "local-whisper-cpp",
        "synthetic",
        SpeechRangeResult {
            language_context: Some(previous),
            range: whisper_backend::SpeechRange {
                start_ms: 12000,
                end_ms: 13000
            },
            fingerprint: "config".into(),
            pending: PendingTranscription {
                base_offset_ms: 12000,
                transcription
            }
        }
    )
    .is_err());
    assert!(read(&storage, &chunk, &track, 17000).is_none());
    storage
        .with_connection(|connection| {
            storage::mark_recording_completed(connection, &chunk.recording_id, 100001)?;
            storage::delete_recording(connection, &chunk.recording_id)?;
            assert_eq!(
                connection.query_row(
                    "SELECT COUNT(*) FROM transcript_language_context",
                    [],
                    |row| row.get::<_, i64>(0)
                )?,
                0
            );
            Ok(())
        })
        .unwrap();
}

#[test]
fn language_context_crosses_finalized_fragments_and_ignores_unpublished_other_jobs() {
    let storage = storage::test_storage(PathBuf::new());
    let (chunk, track) = fixture(&storage);
    save(&storage, &chunk, &track, 98000, None);
    let next = storage
        .with_connection(|connection| {
            let segment_id = Uuid::now_v7().to_string();
            storage::create_recording_segment(
                connection,
                &storage::CreateRecordingSegmentInput {
                    id: segment_id.clone(),
                    recording_id: chunk.recording_id.clone(),
                    segment_index: 2,
                    started_at_ms: 100001,
                    file_path: "/synthetic/second.mkv".into(),
                },
            )?;
            storage::finalize_recording_segment(
                connection,
                &storage::FinalizeRecordingSegmentInput {
                    id: segment_id.clone(),
                    duration_ms: 20000,
                    ended_at_ms: 120001,
                    updated_at_ms: 120001,
                    video_stream_index: None,
                    desktop_audio_stream_index: Some(1),
                    microphone_audio_stream_index: Some(2),
                    probe_json: None,
                },
            )?;
            storage::create_processing_chunk(
                connection,
                &storage::CreateProcessingChunkInput {
                    id: Uuid::now_v7().to_string(),
                    job_id: chunk.job_id.clone(),
                    recording_id: chunk.recording_id.clone(),
                    segment_id: Some(segment_id),
                    track_id: Some(track.id.clone()),
                    source_kind: "audio".into(),
                    start_offset_ms: Some(100000),
                    end_offset_ms: Some(120000),
                    status: "running".into(),
                    attempt_count: 1,
                    progress: 0.0,
                    error_code: None,
                    result_code: None,
                    created_at_ms: 2,
                    updated_at_ms: 2,
                },
            )
        })
        .unwrap();
    let context = read(&storage, &next, &track, 100000).unwrap();
    save(&storage, &next, &track, 100000, Some(context));
    assert_eq!(
        read(&storage, &next, &track, 114000)
            .unwrap()
            .detection_start_ms,
        98000
    );
    storage.with_connection(|connection| {
        let group_id = Uuid::now_v7().to_string();
        connection.execute("INSERT INTO processing_replacement_groups(id,recording_id,job_id,source_track_id,source_role,source_kind,start_ms,end_ms) VALUES(?1,?2,?3,?4,'microphone','audio',98000,101000)", rusqlite::params![group_id,chunk.recording_id,chunk.job_id,track.id])?;
        connection.execute("INSERT INTO processing_replacement_chunks(chunk_id,group_id) VALUES(?1,?2)", rusqlite::params![chunk.id,group_id])?;
        let mut reader = next.clone(); reader.job_id = "another-job".into();
        assert!(storage::preceding_transcript_language(connection, &reader, &track.id, 114000, "config", "local-whisper-cpp", "synthetic")?.is_none());
        connection.execute("UPDATE processing_replacement_groups SET committed=1 WHERE id=?1", [group_id])?;
        assert!(storage::preceding_transcript_language(connection, &reader, &track.id, 114000, "config", "local-whisper-cpp", "synthetic")?.is_some());
        Ok(())
    }).unwrap();
    // Detection can include the preceding fragment's audio halo, even when the
    // new transcript and its owning chunk belong to a different stream index.
    storage
        .with_connection(|connection| {
            connection.execute("DELETE FROM transcript_segments", [])?;
            connection.execute("DELETE FROM processing_audio_ranges", [])?;
            connection.execute(
                "UPDATE recording_segments SET microphone_audio_stream_index=3 WHERE id=?1",
                [next.segment_id.as_ref().unwrap()],
            )?;
            Ok(())
        })
        .unwrap();
    let halo = storage::TranscriptLanguageContext {
        language: "de".into(),
        confidence: 0.9,
        detection_segment_id: None,
        config_fingerprint: "config".into(),
        detection_start_ms: 99500,
        detection_end_ms: 101000,
    };
    save(&storage, &next, &track, 100000, Some(halo));
    assert!(read(&storage, &next, &track, 110000).is_some());
    storage
        .with_connection(|connection| {
            storage::create_redaction(
                connection,
                &storage::CreateRedactionInput {
                    id: Uuid::now_v7().to_string(),
                    recording_id: chunk.recording_id.clone(),
                    start_offset_ms: 99700,
                    end_offset_ms: 99900,
                    created_at_ms: 1,
                    track_ids_json: "[\"microphone-2\"]".into(),
                    reason: None,
                },
            )?;
            Ok(())
        })
        .unwrap();
    assert!(read(&storage, &next, &track, 110000).is_none());
}
