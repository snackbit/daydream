use super::*;

fn fixture() -> (AppStorage, ProcessingChunkRow, RecordingTrackRow) {
    let storage = storage::test_storage(PathBuf::new());
    let (chunk, track) = storage
        .with_connection(|c| Ok(crate::ai::tests::test_transcription_chunk(c)))
        .unwrap();
    (storage, chunk, track)
}
fn unit(
    chunk: &ProcessingChunkRow,
    track: &RecordingTrackRow,
    start: i64,
    end: i64,
) -> storage::SpeechUnit {
    storage::SpeechUnit {
        id: Uuid::now_v7().to_string(),
        job_id: chunk.job_id.clone(),
        track_id: track.id.clone(),
        owner_chunk_id: chunk.id.clone(),
        start_ms: start,
        end_ms: end,
        submitted_start_ms: start,
        submitted_end_ms: end,
        right_open: false,
        status: "queued".into(),
        fingerprint: "synthetic-v2".into(),
        output_json: None,
    }
}
fn stage(storage: &AppStorage, unit: &storage::SpeechUnit) {
    storage
        .with_connection(|c| {
            storage::enable_speech_strategy(c, &unit.job_id)?;
            storage::save_speech_unit(c, unit)?;
            storage::stage_speech_unit(
                c,
                &unit.id,
                &serde_json::to_string(&crate::ai::tests::test_verbose_transcription(
                    0.0,
                    1.0,
                    "Synthetic new",
                    vec![],
                ))
                .unwrap(),
            )
        })
        .unwrap();
}
fn publish(
    storage: &AppStorage,
    chunk: &ProcessingChunkRow,
    track: &RecordingTrackRow,
    unit: &storage::SpeechUnit,
    text: &str,
) -> Result<(), AiError> {
    let settings = settings::load_app_settings(storage).unwrap();
    publication::publish(
        storage,
        chunk,
        track,
        &settings,
        unit,
        crate::ai::tests::test_verbose_transcription(0.0, 1.0, text, vec![]),
    )
}

#[test]
fn published_speech_and_checkpoints_are_atomic_and_idempotent() {
    let (storage, chunk, track) = fixture();
    let unit = unit(&chunk, &track, 0, 1000);
    stage(&storage, &unit);
    assert!(storage
        .with_connection(|c| storage::list_transcript_segments(c, &chunk.recording_id))
        .unwrap()
        .is_empty());
    publish(&storage, &chunk, &track, &unit, "Synthetic new").unwrap();
    publish(&storage, &chunk, &track, &unit, "Duplicate").unwrap();
    let snapshot =
        crate::ai::processing_state_for_recording(&storage, &chunk.recording_id).unwrap();
    assert_eq!(
        snapshot.transcription_progress[0].published_ranges[0].end_offset_ms,
        1000
    );
    assert_eq!(
        snapshot.transcript.tracks[0].segments[0].text,
        "Synthetic new"
    );
    storage
        .with_connection(|c| {
            let units = storage::speech_units(c, &chunk.job_id, &track.id)?;
            assert_eq!(units[0].status, "published");
            assert!(units[0].output_json.is_none());
            assert_eq!(
                storage::list_transcript_segments(c, &chunk.recording_id)?.len(),
                1
            );
            assert!(storage::audio_range_succeeded(
                c,
                &chunk.id,
                0,
                1000,
                "synthetic-v2"
            )?);
            Ok(())
        })
        .unwrap();
}

#[test]
fn rejected_checkpoint_rolls_back_transcript_and_publication() {
    let (storage, chunk, track) = fixture();
    let unit = unit(&chunk, &track, 0, 1000);
    stage(&storage, &unit);
    storage.with_connection(|c|c.execute_batch("CREATE TRIGGER reject_unit_checkpoint BEFORE INSERT ON processing_audio_ranges BEGIN SELECT RAISE(ABORT,'synthetic failure'); END;")).unwrap();
    assert!(publish(&storage, &chunk, &track, &unit, "Synthetic new").is_err());
    storage
        .with_connection(|c| {
            let units = storage::speech_units(c, &chunk.job_id, &track.id)?;
            assert_eq!(units[0].status, "staged");
            assert!(units[0].output_json.is_some());
            assert!(storage::list_transcript_segments(c, &chunk.recording_id)?.is_empty());
            assert_eq!(
                c.query_row("SELECT COUNT(*) FROM transcript_segments", [], |r| r
                    .get::<_, i64>(0))?,
                0
            );
            Ok(())
        })
        .unwrap();
}

#[test]
fn successful_microphone_publication_does_not_wait_for_desktop_audio() {
    let (storage, chunk, track) = fixture();
    let mic = unit(&chunk, &track, 0, 1000);
    stage(&storage, &mic);
    let group = Uuid::now_v7().to_string();
    storage.with_connection(|c|{
        c.execute("INSERT INTO processing_replacement_groups(id,recording_id,job_id,source_track_id,source_role,source_kind,start_ms,end_ms) VALUES(?1,?2,?3,?4,'microphone','audio',0,1000)",rusqlite::params![group,chunk.recording_id,chunk.job_id,track.id])?;
        c.execute("INSERT INTO processing_replacement_chunks(chunk_id,group_id) VALUES(?1,?2)",rusqlite::params![chunk.id,group])?;
        let desktop=storage::upsert_recording_track(c,&UpsertRecordingTrackInput{id:Uuid::now_v7().to_string(),recording_id:chunk.recording_id.clone(),kind:"desktop-audio".into(),label:"Synthetic desktop".into(),codec:None,path:None,source_id:None,created_at_ms:1})?;
        let child=storage::create_processing_chunk(c,&CreateProcessingChunkInput{id:Uuid::now_v7().to_string(),recording_id:chunk.recording_id.clone(),job_id:chunk.job_id.clone(),source_kind:"audio".into(),segment_id:chunk.segment_id.clone(),track_id:Some(desktop.id.clone()),start_offset_ms:Some(0),end_offset_ms:Some(1000),status:"queued".into(),attempt_count:0,progress:0.0,created_at_ms:1,updated_at_ms:1,error_code:None,result_code:None})?;
        storage::save_speech_unit(c,&unit(&child,&desktop,0,1000))?;
        Ok(())
    }).unwrap();
    publish(&storage, &chunk, &track, &mic, "Synthetic microphone").unwrap();
    storage
        .with_connection(|c| {
            storage::update_processing_chunk_status(c, &chunk.id, "succeeded", 1.0, None, 2)?;
            storage::publish_ready_replacement(c, &chunk.id)?;
            assert!(c.query_row(
                "SELECT committed=1 FROM processing_replacement_groups WHERE id=?1",
                [&group],
                |r| r.get::<_, bool>(0)
            )?);
            Ok(())
        })
        .unwrap();
}

#[test]
fn shared_speech_publication_finishes_an_earlier_completed_replacement_group() {
    let (storage, earlier, track) = fixture();
    let group = Uuid::now_v7().to_string();
    let later=storage.with_connection(|c|{
        c.execute("UPDATE processing_chunks SET end_offset_ms=1000,status='succeeded' WHERE id=?1",[&earlier.id])?;
        c.execute("INSERT INTO processing_replacement_groups(id,recording_id,job_id,source_track_id,source_role,source_kind,start_ms,end_ms) VALUES(?1,?2,?3,?4,'microphone','audio',0,1000)",rusqlite::params![group,earlier.recording_id,earlier.job_id,track.id])?;
        c.execute("INSERT INTO processing_replacement_chunks(chunk_id,group_id) VALUES(?1,?2)",rusqlite::params![earlier.id,group])?;
        storage::create_processing_chunk(c,&CreateProcessingChunkInput{id:Uuid::now_v7().to_string(),recording_id:earlier.recording_id.clone(),job_id:earlier.job_id.clone(),source_kind:"audio".into(),segment_id:earlier.segment_id.clone(),track_id:Some(track.id.clone()),start_offset_ms:Some(1000),end_offset_ms:Some(2000),status:"running".into(),attempt_count:1,progress:0.0,created_at_ms:1,updated_at_ms:1,error_code:None,result_code:None})
    }).unwrap();
    let shared = unit(&later, &track, 0, 2000);
    stage(&storage, &shared);
    storage
        .with_connection(|c| {
            storage::publish_ready_replacement(c, &earlier.id)?;
            assert!(!c.query_row(
                "SELECT committed=1 FROM processing_replacement_groups WHERE id=?1",
                [&group],
                |r| r.get::<_, bool>(0)
            )?);
            Ok(())
        })
        .unwrap();
    publish(&storage, &later, &track, &shared, "Synthetic shared speech").unwrap();
    storage
        .with_connection(|c| {
            assert!(c.query_row(
                "SELECT committed=1 FROM processing_replacement_groups WHERE id=?1",
                [&group],
                |r| r.get::<_, bool>(0)
            )?);
            Ok(())
        })
        .unwrap();
}

#[test]
fn old_utterance_is_retained_until_all_replacements_publish() {
    let (storage, old, track) = fixture();
    let current = storage
        .with_connection_mut(|c| {
            transcript_persistence::persist_transcriptions_for_chunk(
                c,
                &old,
                &track,
                "local-whisper-cpp",
                "synthetic",
                old.segment_id.as_ref().unwrap(),
                vec![transcript_persistence::PendingTranscription {
                    base_offset_ms: 0,
                    transcription: crate::ai::tests::test_verbose_transcription(
                        0.0,
                        2.0,
                        "Synthetic old",
                        vec![],
                    ),
                }],
            )?;
            let job = storage::create_processing_job(
                c,
                &CreateProcessingJobInput {
                    id: Uuid::now_v7().to_string(),
                    recording_id: old.recording_id.clone(),
                    kind: AI_PROCESSING_JOB_KIND.into(),
                    status: "running".into(),
                    created_at_ms: 2,
                    updated_at_ms: 2,
                    error: None,
                },
            )?;
            storage::create_processing_chunk(
                c,
                &CreateProcessingChunkInput {
                    id: Uuid::now_v7().to_string(),
                    job_id: job.id,
                    recording_id: old.recording_id.clone(),
                    source_kind: "audio".into(),
                    segment_id: old.segment_id.clone(),
                    track_id: Some(track.id.clone()),
                    start_offset_ms: Some(0),
                    end_offset_ms: Some(10_000),
                    status: "running".into(),
                    attempt_count: 1,
                    progress: 0.0,
                    created_at_ms: 2,
                    updated_at_ms: 2,
                    error_code: None,
                    result_code: None,
                },
            )
        })
        .unwrap();
    let first = unit(&current, &track, 0, 1000);
    let next = unit(&current, &track, 1000, 2000);
    stage(&storage, &first);
    stage(&storage, &next);
    publish(&storage, &current, &track, &first, "Synthetic first").unwrap();
    storage
        .with_connection(|c| {
            assert_eq!(
                c.query_row(
                    "SELECT COUNT(*) FROM transcript_segments WHERE job_id=?1",
                    [&old.job_id],
                    |r| r.get::<_, i64>(0)
                )?,
                1
            );
            let visible = storage::list_transcript_segments(c, &old.recording_id)?;
            assert_eq!(visible.len(), 1);
            assert_eq!(visible[0].text, "Synthetic first");
            Ok(())
        })
        .unwrap();
    publish(&storage, &current, &track, &next, "Synthetic second").unwrap();
    storage
        .with_connection(|c| {
            assert_eq!(
                c.query_row(
                    "SELECT COUNT(*) FROM transcript_segments WHERE job_id=?1",
                    [&old.job_id],
                    |r| r.get::<_, i64>(0)
                )?,
                0
            );
            assert_eq!(
                storage::list_transcript_segments(c, &old.recording_id)?.len(),
                2
            );
            Ok(())
        })
        .unwrap();
}

#[test]
fn changed_privacy_or_attempt_cannot_publish_staged_speech() {
    for stale in [false, true] {
        let (storage, chunk, track) = fixture();
        let unit = unit(&chunk, &track, 0, 1000);
        stage(&storage, &unit);
        storage
            .with_connection(|c| {
                if stale {
                    c.execute(
                        "UPDATE processing_chunks SET attempt_count=attempt_count+1 WHERE id=?1",
                        [&chunk.id],
                    )?;
                } else {
                    storage::create_redaction(
                        c,
                        &storage::CreateRedactionInput {
                            id: Uuid::now_v7().to_string(),
                            recording_id: chunk.recording_id.clone(),
                            start_offset_ms: 100,
                            end_offset_ms: 200,
                            track_ids_json: serde_json::to_string(&vec![track.id.clone()]).unwrap(),
                            reason: None,
                            created_at_ms: 2,
                        },
                    )?;
                }
                Ok(())
            })
            .unwrap();
        assert_eq!(
            publish(&storage, &chunk, &track, &unit, "Synthetic new")
                .unwrap_err()
                .code,
            "ai_processing_cancelled"
        );
        assert!(storage
            .with_connection(|c| storage::list_transcript_segments(c, &chunk.recording_id))
            .unwrap()
            .is_empty());
    }
}

#[test]
fn paused_and_cancelled_jobs_reject_checkpoints_without_becoming_storage_failures() {
    for status in ["paused", "cancelled"] {
        let (storage, chunk, track) = fixture();
        let pending = unit(&chunk, &track, 0, 1000);
        stage(&storage, &pending);
        storage
            .with_connection(|c| {
                storage::update_processing_job_status(c, &chunk.job_id, status, None, 2)
            })
            .unwrap();
        assert_eq!(
            publish(&storage, &chunk, &track, &pending, "Synthetic new")
                .unwrap_err()
                .code,
            "ai_processing_cancelled"
        );
        assert_eq!(
            publication::checkpoint(&storage, &chunk, |_| panic!(
                "Stopped attempts must not write"
            ))
            .unwrap_err()
            .code,
            "ai_processing_cancelled"
        );
        assert_eq!(
            storage
                .with_connection(|c| Ok(storage::get_processing_job(c, &chunk.job_id)?
                    .unwrap()
                    .status))
                .unwrap(),
            status
        );
    }
}

#[test]
fn publication_rechecks_redactions_in_context_outside_the_owning_chunk() {
    let (storage, chunk, track) = fixture();
    let mut pending = unit(&chunk, &track, 0, 1000);
    pending.submitted_end_ms = 2000;
    stage(&storage, &pending);
    storage.with_connection(|c|{
        c.execute("UPDATE processing_chunks SET end_offset_ms=1000 WHERE id=?1",[&chunk.id])?;
        c.execute("UPDATE recording_segments SET duration_ms=2000,timeline_base_ms=0,microphone_audio_stream_index=2 WHERE id=?1",[&chunk.segment_id])?;
        storage::create_redaction(c,&storage::CreateRedactionInput{id:Uuid::now_v7().to_string(),recording_id:chunk.recording_id.clone(),start_offset_ms:1500,end_offset_ms:1700,track_ids_json:"[\"microphone-2\"]".into(),reason:None,created_at_ms:2})?;
        assert!(crate::recording_privacy::chunk_active(c,&chunk)?);Ok(())
    }).unwrap();
    assert_eq!(
        publish(&storage, &chunk, &track, &pending, "Synthetic new")
            .unwrap_err()
            .code,
        "ai_processing_cancelled"
    );
    assert!(storage
        .with_connection(|c| storage::list_transcript_segments(c, &chunk.recording_id))
        .unwrap()
        .is_empty());
}

#[test]
fn retry_finishes_a_pending_boundary_using_the_already_published_neighbor() {
    let (storage, chunk, track) = fixture();
    let mut left = unit(&chunk, &track, 0, 1000);
    left.submitted_end_ms = 1500;
    left.right_open = true;
    let mut right = unit(&chunk, &track, 1000, 2000);
    right.submitted_start_ms = 500;
    stage(&storage, &left);
    stage(&storage, &right);
    let settings = settings::load_app_settings(&storage).unwrap();
    let word = transcription::VerboseTranscriptionWord {
        start: 0.5,
        end: 0.8,
        word: "Boundary".into(),
        probability: None,
    };
    publication::publish(
        &storage,
        &chunk,
        &track,
        &settings,
        &right,
        crate::ai::tests::test_verbose_transcription(0.5, 0.8, "Boundary", vec![word]),
    )
    .unwrap();
    let original = storage
        .with_connection(|c| storage::list_transcript_segments(c, &chunk.recording_id))
        .unwrap()[0]
        .id
        .clone();
    let decoded = crate::ai::tests::test_verbose_transcription(
        0.8,
        1.1,
        "Boundary",
        vec![transcription::VerboseTranscriptionWord {
            start: 0.8,
            end: 1.1,
            word: "Boundary".into(),
            probability: None,
        }],
    );
    assert!(publication::finish_with_published_neighbor(
        &storage, &chunk, &track, &settings, &left, decoded
    )
    .unwrap());
    storage
        .with_connection(|c| {
            assert!(storage::speech_units(c, &chunk.job_id, &track.id)?
                .iter()
                .all(|unit| unit.status == "published"));
            let rows = storage::list_transcript_segments(c, &chunk.recording_id)?;
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0].id, original);
            Ok(())
        })
        .unwrap();
}

#[test]
fn fragment_extraction_preserves_source_time_and_cancels_without_reading() {
    let (storage, chunk, _) = fixture();
    let segment = storage
        .with_connection(
            |c| Ok(storage::list_recording_segments(c, &chunk.recording_id)?.remove(0)),
        )
        .unwrap();
    let directory = TempProcessingDirectory::new(
        std::env::temp_dir().join(format!("daydream-speech-{}", Uuid::now_v7())),
    );
    fs::create_dir_all(directory.path()).unwrap();
    let mut a = segment.clone();
    a.file_path = directory
        .path()
        .join("first.wav")
        .to_string_lossy()
        .into_owned();
    let mut b = segment;
    b.id = Uuid::now_v7().to_string();
    b.file_path = directory
        .path()
        .join("second.wav")
        .to_string_lossy()
        .into_owned();
    whisper_backend::vad::write_detection_samples(Path::new(&a.file_path), &vec![0.25; 8000])
        .unwrap();
    whisper_backend::vad::write_detection_samples(Path::new(&b.file_path), &vec![0.5; 8000])
        .unwrap();
    let run = SourceRun {
        start: 0,
        end: 1000,
        open: false,
        parts: vec![
            input::AudioPart {
                segment: a,
                stream: 0,
                base: 0,
                start: 0,
                end: 500,
            },
            input::AudioPart {
                segment: b,
                stream: 0,
                base: 500,
                start: 500,
                end: 1000,
            },
        ],
    };
    let output = directory.path().join("joined.wav");
    extract_window(&run, 0, 1000, &output, &AtomicBool::new(false)).unwrap();
    let samples = whisper_backend::vad::read_detection_samples(&output).unwrap();
    assert_eq!(samples.len(), 16_000);
    assert!((samples[100] - 0.25).abs() < 0.001);
    assert!((samples[8100] - 0.5).abs() < 0.001);
    assert!(extract_window(&run, 0, 1000, &output, &AtomicBool::new(true)).is_err());
}
