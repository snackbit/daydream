use super::*;
use crate::storage::*;
use uuid::Uuid;

fn db() -> (Connection, super::tests::RecallFixture) {
    let mut db = Connection::open_in_memory().unwrap();
    crate::storage::schema::enable_connection_pragmas(&db).unwrap();
    crate::storage::schema::apply_migrations(&mut db).unwrap();
    let data = super::tests::seed_recall_fixture(&db);
    (db, data)
}

fn crossing_transcript(db: &Connection, f: &super::tests::RecallFixture) -> String {
    let next = Uuid::now_v7().to_string();
    create_recording_segment(
        db,
        &CreateRecordingSegmentInput {
            id: next.clone(),
            recording_id: f.recording.clone(),
            segment_index: 2,
            file_path: f.path.to_string_lossy().into(),
            started_at_ms: 1_010_000,
        },
    )
    .unwrap();
    finalize_recording_segment(
        db,
        &FinalizeRecordingSegmentInput {
            id: next.clone(),
            duration_ms: 10_000,
            ended_at_ms: 1_020_000,
            updated_at_ms: 1_020_000,
            video_stream_index: Some(0),
            desktop_audio_stream_index: Some(3),
            microphone_audio_stream_index: Some(2),
            probe_json: None,
        },
    )
    .unwrap();
    db.execute(
        "UPDATE transcript_segments SET start_offset_ms=9000,end_offset_ms=12000 WHERE id=?1",
        [&f.transcript],
    )
    .unwrap();
    mark_recording_completed(db, &f.recording, 1_020_000).unwrap();
    next
}

#[test]
fn cross_fragment_speech_is_searchable_with_complete_provenance_and_tail_privacy() {
    let (db, f) = db();
    let next = crossing_transcript(&db, &f);
    refresh_evidence(&db).unwrap();
    let items = list_evidence_range(&db, 1_000_000, 1_020_000).unwrap();
    assert_eq!(
        items.len(),
        1,
        "the complete utterance belongs in summary inputs"
    );
    assert_eq!(items[0].citation.segment_id, f.segment);
    assert_eq!(items[0].citation.start_offset_ms, 9000);
    assert_eq!(items[0].citation.end_offset_ms, 12000);
    assert_eq!(items[0].citation.absolute_end_ms, 1_012_000);
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM recall_fts WHERE recall_fts MATCH 'planning'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        1
    );
    let provenance: String = db
        .query_row("SELECT provenance_json FROM recall_evidence", [], |r| {
            r.get(0)
        })
        .unwrap();
    let provenance: serde_json::Value = serde_json::from_str(&provenance).unwrap();
    assert_eq!(provenance["mediaFragments"][0]["segmentId"], f.segment);
    assert_eq!(provenance["mediaFragments"][1]["segmentId"], next);
    create_redaction(
        &db,
        &CreateRedactionInput {
            id: Uuid::now_v7().to_string(),
            recording_id: f.recording.clone(),
            start_offset_ms: 11000,
            end_offset_ms: 11500,
            track_ids_json: "[\"desktop-audio-3\"]".into(),
            created_at_ms: 2,
            reason: None,
        },
    )
    .unwrap();
    refresh_evidence(&db).unwrap();
    assert!(list_evidence_range(&db, 0, i64::MAX).unwrap().is_empty());
}

#[test]
fn cross_fragment_evidence_withholds_unavailable_or_discontinuous_support() {
    for change in [
        "desktop_audio_stream_index=NULL",
        "timeline_base_ms=10001",
        "timeline_basis_version=NULL",
        "started_at_ms=1010001",
        "state='failed'",
        "file_path='/tmp/daydream-missing-cross-fragment-test-source'",
    ] {
        let (db, f) = db();
        let tail = crossing_transcript(&db, &f);
        refresh_evidence(&db).unwrap();
        assert_eq!(list_evidence_range(&db, 0, i64::MAX).unwrap().len(), 1);
        db.execute(
            &format!("UPDATE recording_segments SET {change} WHERE id=?1"),
            [&tail],
        )
        .unwrap();
        refresh_evidence(&db).unwrap();
        assert!(
            list_evidence_range(&db, 0, i64::MAX).unwrap().is_empty(),
            "invalid support: {change}"
        );
    }
}

#[test]
fn silent_publication_invalidates_hidden_historical_search_vectors_and_summaries() {
    let (mut db, f) = db();
    let job = Uuid::now_v7().to_string();
    create_processing_job(
        &db,
        &CreateProcessingJobInput {
            id: job.clone(),
            recording_id: f.recording.clone(),
            kind: "transcription".into(),
            status: "running".into(),
            created_at_ms: 2,
            updated_at_ms: 2,
            error: None,
        },
    )
    .unwrap();
    let chunk = create_processing_chunk(
        &db,
        &CreateProcessingChunkInput {
            id: Uuid::now_v7().to_string(),
            recording_id: f.recording.clone(),
            job_id: job.clone(),
            segment_id: Some(f.segment.clone()),
            source_kind: "audio".into(),
            track_id: Some(f.track.clone()),
            start_offset_ms: Some(1000),
            end_offset_ms: Some(2000),
            status: "running".into(),
            attempt_count: 1,
            progress: 0.0,
            created_at_ms: 2,
            updated_at_ms: 2,
            error_code: None,
            result_code: None,
        },
    )
    .unwrap();
    let group = Uuid::now_v7().to_string();
    db.execute("INSERT INTO processing_replacement_groups(id,recording_id,job_id,source_track_id,source_role,source_kind,start_ms,end_ms) VALUES(?1,?2,?3,?4,'desktop-audio','audio',1000,2000)", params![group,f.recording,job,f.track]).unwrap();
    db.execute(
        "INSERT INTO processing_replacement_chunks VALUES(?1,?2)",
        params![chunk.id, group],
    )
    .unwrap();
    let track = list_recording_tracks(&db, &f.recording).unwrap().remove(0);
    let left = SpeechUnit {
        id: Uuid::now_v7().to_string(),
        job_id: job.clone(),
        track_id: track.id.clone(),
        owner_chunk_id: chunk.id.clone(),
        start_ms: 1000,
        end_ms: 1500,
        submitted_start_ms: 1000,
        submitted_end_ms: 1500,
        right_open: false,
        status: "queued".into(),
        fingerprint: "synthetic-silence".into(),
        output_json: None,
    };
    let right = SpeechUnit {
        id: Uuid::now_v7().to_string(),
        start_ms: 1500,
        end_ms: 2000,
        submitted_start_ms: 1500,
        submitted_end_ms: 2000,
        ..left.clone()
    };
    save_speech_unit(&db, &left).unwrap();
    save_speech_unit(&db, &right).unwrap();
    refresh_evidence(&db).unwrap();
    let evidence = list_evidence_range(&db, 0, i64::MAX).unwrap().remove(0);
    db.execute(
        "INSERT INTO recall_index VALUES(?1,?2,'synthetic-space',1,?3)",
        params![
            evidence.citation.evidence_id,
            evidence.citation.content_revision,
            1.0_f32.to_le_bytes().to_vec()
        ],
    )
    .unwrap();
    db.execute(
        "INSERT INTO recall_generations VALUES('day','f','p','{}','ready',0)",
        [],
    )
    .unwrap();
    db.execute(
        "INSERT INTO recall_generation_inputs VALUES('day',?1,?2)",
        params![
            evidence.citation.evidence_id,
            evidence.citation.content_revision
        ],
    )
    .unwrap();
    let revision = privacy_revision(&db).unwrap();
    {
        let tx = db.transaction().unwrap();
        publish_speech_unit(&tx, &left, &chunk, &track).unwrap();
        refresh_evidence(&tx).unwrap();
        assert!(read_evidence(&tx, &evidence.citation.evidence_id)
            .unwrap()
            .is_none());
        // Rolling back publication restores vectors, FTS and summary lineage.
    }
    assert_eq!(privacy_revision(&db).unwrap(), revision);
    assert!(read_evidence(&db, &evidence.citation.evidence_id)
        .unwrap()
        .is_some());
    {
        let tx = db.transaction().unwrap();
        publish_speech_unit(&tx, &left, &chunk, &track).unwrap();
        tx.commit().unwrap();
    }
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM transcript_segments WHERE id=?1",
            [&f.transcript],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        1,
        "raw history stays until the remaining unit publishes"
    );
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM published_transcript_segments WHERE id=?1",
            [&f.transcript],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    refresh_evidence(&db).unwrap();
    assert!(read_evidence(&db, &evidence.citation.evidence_id)
        .unwrap()
        .is_none());
    assert_eq!(
        db.query_row("SELECT count(*) FROM recall_fts", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(
        db.query_row("SELECT count(*) FROM recall_index", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(
        db.query_row(
            "SELECT status FROM recall_generations WHERE day_key='day'",
            [],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        "invalidated"
    );
}
