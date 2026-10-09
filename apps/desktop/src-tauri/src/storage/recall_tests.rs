use super::*;
use crate::storage::*;
use uuid::Uuid;

fn id() -> String {
    Uuid::now_v7().to_string()
}
struct Fixture {
    connection: Connection,
    data: RecallFixture,
}
pub(crate) struct RecallFixture {
    pub recording: String,
    pub segment: String,
    pub path: std::path::PathBuf,
    pub track: String,
    pub transcript: String,
}
impl std::ops::Deref for Fixture {
    type Target = RecallFixture;
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}
impl Drop for RecallFixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}
fn fixture() -> Fixture {
    let mut connection = Connection::open_in_memory().unwrap();
    crate::storage::schema::enable_connection_pragmas(&connection).unwrap();
    crate::storage::schema::apply_migrations(&mut connection).unwrap();
    let data = seed_recall_fixture(&connection);
    Fixture { connection, data }
}
pub(crate) fn seed_recall_fixture(connection: &Connection) -> RecallFixture {
    let recording = id();
    let segment = id();
    let job = id();
    let chunk = id();
    let track = id();
    let transcript = id();
    let path = std::env::temp_dir().join(format!("daydream-recall-synthetic-{}.mkv", id()));
    std::fs::write(&path, b"synthetic").unwrap();
    create_recording(
        connection,
        &CreateRecordingInput {
            id: recording.clone(),
            active_scene_id: None,
            created_at_ms: 0,
            default_fps: 1.,
            notes: None,
            output_root: None,
            quality_label: "balanced-ocr".into(),
            storage_dir: "/tmp/synthetic-recall".into(),
        },
    )
    .unwrap();
    create_recording_segment(
        connection,
        &CreateRecordingSegmentInput {
            id: segment.clone(),
            file_path: path.to_string_lossy().into(),
            recording_id: recording.clone(),
            segment_index: 1,
            started_at_ms: 1_000_000,
        },
    )
    .unwrap();
    finalize_recording_segment(
        connection,
        &FinalizeRecordingSegmentInput {
            id: segment.clone(),
            duration_ms: 10_000,
            ended_at_ms: 1_010_000,
            updated_at_ms: 1_010_000,
            desktop_audio_stream_index: Some(1),
            microphone_audio_stream_index: Some(2),
            video_stream_index: Some(0),
            probe_json: None,
        },
    )
    .unwrap();
    mark_recording_started(connection, &recording, 1_000_000).unwrap();
    mark_recording_completed(connection, &recording, 1_010_000).unwrap();
    upsert_recording_track(
        connection,
        &UpsertRecordingTrackInput {
            id: track.clone(),
            recording_id: recording.clone(),
            kind: "desktop-audio".into(),
            label: "Synthetic".into(),
            codec: None,
            path: None,
            source_id: None,
            created_at_ms: 0,
        },
    )
    .unwrap();
    create_processing_job(
        connection,
        &CreateProcessingJobInput {
            id: job.clone(),
            recording_id: recording.clone(),
            kind: "transcription".into(),
            status: "running".into(),
            created_at_ms: 0,
            updated_at_ms: 0,
            error: None,
        },
    )
    .unwrap();
    create_processing_chunk(
        connection,
        &CreateProcessingChunkInput {
            id: chunk.clone(),
            recording_id: recording.clone(),
            job_id: job.clone(),
            segment_id: Some(segment.clone()),
            source_kind: "audio".into(),
            track_id: Some(track.clone()),
            start_offset_ms: Some(0),
            end_offset_ms: Some(10_000),
            attempt_count: 1,
            created_at_ms: 0,
            updated_at_ms: 0,
            progress: 0.,
            status: "running".into(),
            error_code: None,
            result_code: None,
        },
    )
    .unwrap();
    create_transcript_segment(
        connection,
        &CreateTranscriptSegmentInput {
            id: transcript.clone(),
            recording_id: recording.clone(),
            segment_id: segment.clone(),
            track_id: track.clone(),
            source_role: "desktop-audio".into(),
            start_offset_ms: 1000,
            end_offset_ms: 2000,
            provider: "synthetic".into(),
            model: None,
            text: "Synthetic planning note".into(),
            confidence: None,
            job_id: job,
            chunk_id: chunk,
            created_at_ms: 1,
        },
    )
    .unwrap();
    RecallFixture {
        recording,
        segment,
        path,
        track,
        transcript,
    }
}

#[test]
fn privacy_mutation_immediately_removes_search_and_uncited_lineage() {
    let f = fixture();
    let connection = &f.connection;
    let revision = refresh_evidence(connection).unwrap();
    let evidence = list_evidence_range(connection, 0, i64::MAX).unwrap();
    assert_eq!(evidence.len(), 1);
    assert_eq!(evidence[0].citation.absolute_start_ms, 1_001_000);
    assert_eq!(
        connection
            .query_row("SELECT count(*) FROM recall_fts", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        1
    );
    connection
        .execute(
            "INSERT INTO recall_generations VALUES('1970-01-01@UTC','f','p','{}','ready',0)",
            [],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO recall_generation_inputs VALUES('1970-01-01@UTC',?1,?2)",
            params![
                evidence[0].citation.evidence_id,
                evidence[0].citation.content_revision
            ],
        )
        .unwrap();
    create_redaction(
        connection,
        &CreateRedactionInput {
            id: id(),
            recording_id: f.recording.clone(),
            start_offset_ms: 1000,
            end_offset_ms: 1500,
            track_ids_json: format!("[\"{}\"]", f.track),
            created_at_ms: 0,
            reason: None,
        },
    )
    .unwrap();
    assert!(privacy_revision(connection).unwrap() > revision);
    assert_eq!(
        connection
            .query_row("SELECT status FROM recall_generations", [], |row| row
                .get::<_, String>(0))
            .unwrap(),
        "invalidated"
    );
    assert_eq!(
        connection
            .query_row("SELECT count(*) FROM recall_fts", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_ne!(refresh_evidence(connection).unwrap(), revision);
    assert!(list_evidence_range(connection, 0, i64::MAX)
        .unwrap()
        .is_empty());
}

#[test]
fn nonmatching_track_is_visible_and_missing_timing_is_withheld() {
    let f = fixture();
    let connection = &f.connection;
    create_redaction(
        connection,
        &CreateRedactionInput {
            id: id(),
            recording_id: f.recording.clone(),
            start_offset_ms: 0,
            end_offset_ms: 5000,
            track_ids_json: "[\"video-0\"]".into(),
            created_at_ms: 0,
            reason: None,
        },
    )
    .unwrap();
    let revision = refresh_evidence(connection).unwrap();
    assert_eq!(
        list_evidence_range(connection, 0, i64::MAX).unwrap().len(),
        1
    );
    assert_eq!(refresh_evidence(connection).unwrap(), revision);
    connection.execute("UPDATE recording_segments SET timeline_base_ms=NULL,timeline_basis_version=NULL WHERE id=?1",[&f.segment]).unwrap();
    refresh_evidence(connection).unwrap();
    assert!(
        read_evidence(connection, &format!("transcript:{}", f.transcript))
            .unwrap()
            .is_none()
    );
}

#[test]
fn durable_successor_base_survives_missing_predecessor_and_legacy_is_conservative() {
    let f = fixture();
    let connection = &f.connection;
    let next = id();
    let row = create_recording_segment(
        connection,
        &CreateRecordingSegmentInput {
            id: next.clone(),
            recording_id: f.recording.clone(),
            file_path: "/tmp/missing-synthetic.mkv".into(),
            segment_index: 2,
            started_at_ms: 1_015_000,
        },
    )
    .unwrap();
    assert_eq!(row.timeline_base_ms, Some(15_000));
    std::fs::remove_file(&f.path).unwrap();
    let segments = list_recording_segments(connection, &f.recording).unwrap();
    assert_eq!(segment_playback_bases(&segments), vec![0, 15_000]);
    connection.execute("UPDATE recording_segments SET timeline_base_ms=NULL,timeline_basis_version=NULL WHERE recording_id=?1",[&f.recording]).unwrap();
    backfill_segment_timeline_bases(connection).unwrap();
    assert!(list_recording_segments(connection, &f.recording)
        .unwrap()
        .iter()
        .all(|segment| segment.timeline_base_ms.is_none()));
    std::fs::write(&f.path, b"synthetic").unwrap();
    backfill_segment_timeline_bases(connection).unwrap();
    backfill_segment_timeline_bases(connection).unwrap();
    assert_eq!(
        list_recording_segments(connection, &f.recording).unwrap()[1].timeline_base_ms,
        Some(15_000)
    );
}

#[test]
fn day_coverage_clips_midnight_unions_overlap_and_distinguishes_missing_media() {
    let f = fixture();
    let connection = &f.connection;
    let second = seed_recall_fixture(connection);
    let input = crate::recall::types::RecallDayInput {
        day_id: "2026-10-25".into(),
        timezone: "Europe/Berlin".into(),
    };
    let (start, end) = crate::recall::day::day_bounds(&input.day_id, &input.timezone).unwrap();
    assert_eq!(end - start, 25 * 3_600_000);
    connection.execute("UPDATE recording_segments SET started_at_ms=?2,ended_at_ms=?3,duration_ms=?4 WHERE id=?1",params![f.segment,start-600_000,start+1_800_000,2_400_000]).unwrap();
    connection.execute("UPDATE recording_segments SET started_at_ms=?2,ended_at_ms=?3,duration_ms=?4 WHERE id=?1",params![second.segment,start+600_000,start+2_400_000,1_800_000]).unwrap();
    let day = crate::recall::day::base_day(connection, &input).unwrap();
    assert_eq!(day.day.recording_count, 2);
    assert_eq!(day.day.captured_duration_ms, 2_400_000);
    assert!(day
        .coverage
        .iter()
        .all(|interval| interval.start_ms >= start && interval.end_ms <= end));
    std::fs::remove_file(&f.path).unwrap();
    let day = crate::recall::day::base_day(connection, &input).unwrap();
    // Durable recording timing remains available after physical media is missing.
    assert_eq!(day.day.captured_duration_ms, 2_400_000);
    assert_eq!(day.recordings.len(), 2);
    assert_eq!(
        day.recordings
            .iter()
            .find(|row| row.recording_id == f.recording)
            .unwrap()
            .start_offset_ms,
        Some(600_000)
    );
    assert!(day
        .coverage
        .iter()
        .any(|interval| interval.kind == "missing" && interval.recording_id == f.recording));
    assert!(day
        .coverage
        .iter()
        .any(|interval| interval.kind == "gap" && interval.start_ms == start + 2_400_000));
}

#[test]
fn physical_deletion_after_hide_purges_generated_support_but_keeps_user_edits() {
    let f = fixture();
    let connection = &f.connection;
    refresh_evidence(connection).unwrap();
    let evidence = list_evidence_range(connection, 0, i64::MAX).unwrap();
    let source_id = &evidence[0].citation.evidence_id;
    connection
        .execute(
            "INSERT INTO recall_generations VALUES('day','f','p','{}','ready',0)",
            [],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO recall_generation_inputs VALUES('day',?1,?2)",
            params![
                evidence[0].citation.evidence_id,
                evidence[0].citation.content_revision
            ],
        )
        .unwrap();
    let sources=serde_json::json!({"inputs":[{"id":source_id,"revision":evidence[0].citation.content_revision}],"citations":[]}).to_string();
    connection.execute("INSERT INTO recall_tasks(id,day_key,action_key,source_json,title,edited_title,notes,status,updated_at) VALUES('task','day','key',?1,'Generated private title','User-authored title','User note','done',0)",[sources]).unwrap();
    create_redaction(
        connection,
        &CreateRedactionInput {
            id: id(),
            recording_id: f.recording.clone(),
            start_offset_ms: 0,
            end_offset_ms: 5000,
            track_ids_json: "[]".into(),
            created_at_ms: 0,
            reason: None,
        },
    )
    .unwrap();
    connection
        .execute("DELETE FROM recordings WHERE id=?1", [&f.recording])
        .unwrap();
    let (title, edited, notes, status, source): (String, String, String, String, String) =
        connection
            .query_row(
                "SELECT title,edited_title,notes,status,source_json FROM recall_tasks",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .unwrap();
    assert!(title.is_empty());
    assert_eq!(edited, "User-authored title");
    assert_eq!(notes, "User note");
    assert_eq!(status, "done");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&source).unwrap()["withheld"],
        true
    );
    assert_eq!(
        connection
            .query_row("SELECT count(*) FROM recall_generation_inputs", [], |row| {
                row.get::<_, i64>(0)
            })
            .unwrap(),
        0
    );
}

#[test]
fn both_audio_roles_retain_identity_and_track_specific_redaction() {
    let f = fixture();
    let connection = &f.connection;
    let microphone_track = id();
    let microphone_chunk = id();
    let microphone_transcript = id();
    connection.execute("INSERT INTO recording_tracks(id,recording_id,kind,label,created_at_ms) VALUES(?1,?2,'microphone','Synthetic mic',0)",params![microphone_track,f.recording]).unwrap();
    connection.execute("INSERT INTO processing_chunks(id,job_id,recording_id,source_kind,segment_id,start_offset_ms,end_offset_ms,status,attempt_count,progress,created_at_ms,updated_at_ms,track_id) SELECT ?1,job_id,recording_id,'audio',segment_id,0,10000,'running',1,0,0,0,?2 FROM processing_chunks LIMIT 1",params![microphone_chunk,microphone_track]).unwrap();
    connection.execute("INSERT INTO transcript_segments(id,recording_id,segment_id,track_id,source_role,start_offset_ms,end_offset_ms,provider,text,job_id,chunk_id,created_at_ms) SELECT ?1,recording_id,segment_id,?2,'microphone',1000,2000,'synthetic','Synthetic spoken note',job_id,?3,1 FROM transcript_segments LIMIT 1",params![microphone_transcript,microphone_track,microphone_chunk]).unwrap();
    refresh_evidence(connection).unwrap();
    let evidence = list_evidence_range(connection, 0, i64::MAX).unwrap();
    assert_eq!(evidence.len(), 2);
    assert!(evidence
        .iter()
        .any(|item| item.citation.source_kind == "microphone"
            && item.citation.source_id == microphone_transcript
            && item.citation.source_track_id == "microphone-2"));
    assert!(evidence
        .iter()
        .any(|item| item.citation.source_kind == "desktop-audio"
            && item.citation.source_track_id == "desktop-audio-1"));
    create_redaction(
        connection,
        &CreateRedactionInput {
            id: id(),
            recording_id: f.recording.clone(),
            start_offset_ms: 0,
            end_offset_ms: 3000,
            track_ids_json: "[\"desktop-audio-1\"]".into(),
            created_at_ms: 0,
            reason: None,
        },
    )
    .unwrap();
    refresh_evidence(connection).unwrap();
    let evidence = list_evidence_range(connection, 0, i64::MAX).unwrap();
    assert_eq!(evidence.len(), 1);
    assert_eq!(evidence[0].citation.source_kind, "microphone");
}

#[test]
fn current_ocr_documents_group_exact_occurrences_and_reject_stale_attempts() {
    let mut f = fixture();
    let job = id();
    let chunk_id = id();
    let chunk = create_processing_chunk(
        &f.connection,
        &CreateProcessingChunkInput {
            id: chunk_id,
            recording_id: f.recording.clone(),
            job_id: {
                create_processing_job(
                    &f.connection,
                    &CreateProcessingJobInput {
                        id: job.clone(),
                        recording_id: f.recording.clone(),
                        kind: "ocr".into(),
                        status: "running".into(),
                        created_at_ms: 0,
                        updated_at_ms: 0,
                        error: None,
                    },
                )
                .unwrap();
                job
            },
            segment_id: Some(f.segment.clone()),
            source_kind: "ocr".into(),
            track_id: None,
            start_offset_ms: Some(0),
            end_offset_ms: Some(10_000),
            attempt_count: 1,
            created_at_ms: 0,
            updated_at_ms: 0,
            progress: 0.,
            status: "running".into(),
            error_code: None,
            result_code: None,
        },
    )
    .unwrap();
    let first = id();
    for (index, timestamp) in [2000, 3000].into_iter().enumerate() {
        let frame = OcrFrameInput {
            id: if index == 0 { first.clone() } else { id() },
            frame_timestamp_ms: timestamp,
            sample_end_ms: timestamp + 1000,
            source_frame_ref: format!("synthetic-{timestamp}"),
            source_track_id: "video-0".into(),
            width: 100,
            height: 100,
            config_fingerprint: "synthetic-ocr".into(),
            engine: "tesseract".into(),
            provider: "local-tesseract".into(),
            model: Some("eng".into()),
            pixel_hash: format!("synthetic-{timestamp}"),
            status: "succeeded".into(),
            error_code: None,
            content: Some(OcrContentInput {
                text: "Synthetic exact document".into(),
                blocks: vec![],
            }),
            cache_content_id: None,
            raw_regions: vec![],
        };
        assert!(save_ocr_frame_result(&mut f.connection, &chunk, &frame).unwrap());
    }
    refresh_evidence(&f.connection).unwrap();
    let evidence = list_evidence_range(&f.connection, 0, i64::MAX).unwrap();
    let ocr = evidence
        .iter()
        .filter(|item| item.citation.source_kind == "ocr")
        .collect::<Vec<_>>();
    assert_eq!(ocr.len(), 1);
    assert_eq!(ocr[0].citation.start_offset_ms, 2000);
    assert_eq!(ocr[0].citation.end_offset_ms, 4000);
    assert_eq!(ocr[0].citation.document_id, Some(first));
    assert_eq!(
        ocr[0].citation.source_frame_ref,
        Some("synthetic-2000".into())
    );
    f.connection
        .execute(
            "UPDATE processing_chunks SET attempt_count=2 WHERE id=?1",
            [chunk.id],
        )
        .unwrap();
    refresh_evidence(&f.connection).unwrap();
    assert!(list_evidence_range(&f.connection, 0, i64::MAX)
        .unwrap()
        .iter()
        .all(|item| item.citation.source_kind != "ocr"));
}

#[test]
fn ordinary_source_growth_preserves_unchanged_vectors_and_generation_windows() {
    let f = fixture();
    let connection = &f.connection;
    refresh_evidence(connection).unwrap();
    let evidence = list_evidence_range(connection, 0, i64::MAX).unwrap();
    let first = &evidence[0];
    connection
        .execute(
            "INSERT INTO recall_index VALUES(?1,?2,'synthetic-space',1,?3)",
            params![
                first.citation.evidence_id,
                first.citation.content_revision,
                1.0_f32.to_le_bytes().to_vec()
            ],
        )
        .unwrap();
    let lineage=serde_json::json!([{"id":first.citation.evidence_id,"revision":first.citation.content_revision}]).to_string();
    connection
        .execute(
            "INSERT INTO recall_generation_windows VALUES('day','window','profile','{}',?1,0)",
            [lineage],
        )
        .unwrap();
    connection
        .execute(
            "UPDATE processing_chunks SET status='succeeded' WHERE recording_id=?1",
            [&f.recording],
        )
        .unwrap();
    refresh_evidence(connection).unwrap();
    let _second = seed_recall_fixture(connection);
    refresh_evidence(connection).unwrap();
    assert_eq!(
        list_evidence_range(connection, 0, i64::MAX).unwrap().len(),
        2
    );
    assert_eq!(
        connection
            .query_row("SELECT count(*) FROM recall_index", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        connection
            .query_row(
                "SELECT count(*) FROM recall_generation_windows",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
        1
    );
    assert_eq!(
        connection
            .query_row("SELECT count(*) FROM recall_fts", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        2
    );
    connection
        .execute(
            "UPDATE processing_jobs SET status='cancelled' WHERE recording_id=?1",
            [&f.recording],
        )
        .unwrap();
    refresh_evidence(connection).unwrap();
    assert_eq!(
        list_evidence_range(connection, 0, i64::MAX).unwrap().len(),
        1
    );
    assert_eq!(
        connection
            .query_row("SELECT count(*) FROM recall_index", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(
        connection
            .query_row(
                "SELECT count(*) FROM recall_generation_windows",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
        0
    );
}

#[test]
fn long_source_fragments_retain_parent_and_exact_timing_without_source_gaps() {
    let f = fixture();
    let connection = &f.connection;
    let text = format!(
        "{} The synthetic rare clause identifies inverter Rückkopplung. {}",
        "Synthetic routine sentence. ".repeat(100),
        "Weitere gewöhnliche deutsche Wörter. ".repeat(100)
    );
    connection
        .execute(
            "UPDATE transcript_segments SET text=?2 WHERE id=?1",
            params![f.transcript, text],
        )
        .unwrap();
    refresh_evidence(connection).unwrap();
    let evidence = list_evidence_range(connection, 0, i64::MAX).unwrap();
    assert!(evidence.len() > 3);
    assert!(evidence
        .iter()
        .all(|item| item.citation.source_id == f.transcript
            && item.citation.evidence_id.contains("#fragment:")
            && item.citation.absolute_start_ms == 1_001_000
            && item.citation.absolute_end_ms == 1_002_000
            && item.text.len() <= 480
            && text.contains(&item.text)));
    assert!(evidence
        .iter()
        .any(|item| item.text.contains("inverter Rückkopplung")));
    create_redaction(
        connection,
        &CreateRedactionInput {
            id: id(),
            recording_id: f.recording.clone(),
            start_offset_ms: 1100,
            end_offset_ms: 1200,
            track_ids_json: "[]".into(),
            created_at_ms: 0,
            reason: None,
        },
    )
    .unwrap();
    refresh_evidence(connection).unwrap();
    assert!(list_evidence_range(connection, 0, i64::MAX)
        .unwrap()
        .is_empty());
}
