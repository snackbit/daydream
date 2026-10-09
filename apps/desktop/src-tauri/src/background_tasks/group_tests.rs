use super::super::group_test_fixtures::*;
use super::*;

#[test]
fn grouping_precedes_pagination_and_attention_filter_keeps_successful_siblings() {
    let storage = fixture();
    storage
        .with_connection(|connection| {
            recording(connection, "a");
            recording(connection, "b");
            job(connection, "a", "audio-a", 20, "audio", "succeeded");
            job(connection, "a", "ocr-a", 30, "ocr", "failed");
            job(connection, "b", "audio-b", 10, "audio", "succeeded");
            Ok(())
        })
        .unwrap();
    let first = list(
        ListRecordingProcessingGroupsInput {
            limit: Some(1),
            ..Default::default()
        },
        &storage,
    )
    .unwrap();
    assert_eq!(first.groups.len(), 1);
    assert_eq!(first.groups[0].recording_id, fixture_id("a"));
    assert_eq!(
        first.groups[0]
            .lanes
            .iter()
            .filter(|lane| lane.total > 0)
            .count(),
        2
    );
    assert_eq!(first.counts.all, 2);
    assert_eq!(first.counts.attention, 1);
    let next = list(
        ListRecordingProcessingGroupsInput {
            limit: Some(1),
            cursor: first.next_cursor,
            ..Default::default()
        },
        &storage,
    )
    .unwrap();
    assert_eq!(next.groups[0].recording_id, fixture_id("b"));
    assert!(next.next_cursor.is_none());
    let attention = list(
        ListRecordingProcessingGroupsInput {
            filter: Some("attention".into()),
            ..Default::default()
        },
        &storage,
    )
    .unwrap();
    assert_eq!(attention.groups.len(), 1);
    assert_eq!(attention.groups[0].lanes[0].completed, 1);
}

#[test]
fn superseded_failures_are_not_summed_and_open_input_never_has_finish_eta() {
    let storage = fixture();
    storage
        .with_connection(|connection| {
            recording(connection, "a");
            job(connection, "a", "old", 10, "ocr", "failed");
            job(connection, "a", "new", 20, "ocr", "succeeded");
            storage::set_processing_job_runtime(connection, &fixture_id("new"), true, "{}")?;
            Ok(())
        })
        .unwrap();
    let result = list(Default::default(), &storage).unwrap();
    let group = &result.groups[0];
    let lane = &group.lanes[1];
    assert_eq!(lane.total, 1);
    assert_eq!(lane.failed, 0);
    assert_eq!(lane.job_ids, [fixture_id("new")]);
    assert!(group.input_open);
    assert_eq!(group.status, "running");
    assert!(group.eta_ms.is_none());
    assert_eq!(result.counts.attention, 0);
}

#[test]
fn cancelled_and_skipped_are_disjoint_and_reconciliation_removes_deleted_recordings() {
    let storage = fixture();
    storage
        .with_connection(|connection| {
            recording(connection, "a");
            job(connection, "a", "audio", 10, "audio", "succeeded");
            connection.execute(
                "UPDATE processing_chunks SET result_code='skipped-silence' WHERE job_id=?1",
                [fixture_id("audio")],
            )?;
            job(connection, "a", "ocr", 20, "ocr", "cancelled");
            Ok(())
        })
        .unwrap();
    let group = list(Default::default(), &storage).unwrap().groups.remove(0);
    assert_eq!(group.lanes[0].completed, 0);
    assert_eq!(group.lanes[0].skipped, 1);
    assert_eq!(group.lanes[1].cancelled, 1);
    assert_ne!(group.status, "succeeded");
    storage
        .with_connection(|connection| {
            storage::delete_recording(connection, &fixture_id("a"))?;
            Ok(())
        })
        .unwrap();
    let page = list(
        ListRecordingProcessingGroupsInput {
            known_recording_ids: vec![fixture_id("a")],
            ..Default::default()
        },
        &storage,
    )
    .unwrap();
    assert!(page.groups.is_empty());
    assert!(page.reconciled_groups.is_empty());
}

#[test]
fn lane_retry_changes_only_the_selected_source_kind() {
    let storage = fixture();
    storage
        .with_connection(|connection| {
            recording(connection, "a");
            job(connection, "a", "mixed", 10, "audio", "failed");
            storage::create_processing_chunk(
                connection,
                &storage::CreateProcessingChunkInput {
                    id: fixture_id("mixed-ocr"),
                    job_id: fixture_id("mixed"),
                    recording_id: fixture_id("a"),
                    source_kind: "ocr".into(),
                    segment_id: None,
                    track_id: None,
                    start_offset_ms: Some(0),
                    end_offset_ms: Some(1000),
                    attempt_count: 1,
                    created_at_ms: 10,
                    updated_at_ms: 10,
                    status: "failed".into(),
                    progress: 1.0,
                    error_code: None,
                    result_code: None,
                },
            )?;
            Ok(())
        })
        .unwrap();
    crate::ai::retry_processing_units_for_action(
        &storage,
        &crate::ai::RetryProcessingUnitsInput {
            job_id: fixture_id("mixed"),
            chunk_id: None,
            unit_id: None,
            source_kind: Some("ocr".into()),
            use_current_settings: false,
        },
    )
    .unwrap();
    storage
        .with_connection(|connection| {
            let chunks = storage::list_processing_chunks(connection, &fixture_id("mixed"))?;
            assert_eq!(
                chunks
                    .iter()
                    .find(|chunk| chunk.source_kind == "audio")
                    .unwrap()
                    .status,
                "failed"
            );
            assert_eq!(
                chunks
                    .iter()
                    .find(|chunk| chunk.source_kind == "ocr")
                    .unwrap()
                    .status,
                "queued"
            );
            Ok(())
        })
        .unwrap();
}

#[test]
fn index_coverage_uses_current_eligible_revision_and_embedding_space_per_recording() {
    let storage = fixture();
    let (first, _second) = storage
        .with_connection(|connection| {
            Ok((
                storage::seed_recall_fixture(connection),
                storage::seed_recall_fixture(connection),
            ))
        })
        .unwrap();
    let initial = list(Default::default(), &storage).unwrap();
    let before = initial
        .groups
        .iter()
        .find(|group| group.recording_id == first.recording)
        .unwrap();
    assert!(before.index_coverage.total > 0);
    assert_eq!(before.index_coverage.indexed, 0);
    let settings = settings::load_app_settings(&storage).unwrap();
    let space = crate::recall::embedding::configured_space(&settings.ai.recall.embedding).unwrap();
    storage.with_connection(|connection|{
        connection.execute("INSERT INTO recall_index(evidence_id,source_revision,embedding_space,dimensions,vector) SELECT id,revision,?2,384,X'00000000' FROM recall_evidence WHERE recording_id=?1",params![first.recording,space.fingerprint])?;Ok(())
    }).unwrap();
    let indexed = list(Default::default(), &storage).unwrap();
    assert_eq!(
        indexed
            .groups
            .iter()
            .find(|group| group.recording_id == first.recording)
            .unwrap()
            .index_coverage
            .pending,
        0
    );
    assert!(indexed
        .groups
        .iter()
        .filter(|group| group.recording_id != first.recording)
        .all(|group| group.index_coverage.indexed == 0));
    storage
        .with_connection(|connection| {
            connection.execute(
                "UPDATE recall_index SET embedding_space='synthetic-previous-space'",
                [],
            )?;
            Ok(())
        })
        .unwrap();
    let changed = list(Default::default(), &storage).unwrap();
    assert_eq!(
        changed
            .groups
            .iter()
            .find(|group| group.recording_id == first.recording)
            .unwrap()
            .index_coverage
            .indexed,
        0
    );
    storage
        .with_connection(|connection| {
            storage::create_redaction(
                connection,
                &storage::CreateRedactionInput {
                    id: uuid::Uuid::now_v7().to_string(),
                    recording_id: first.recording.clone(),
                    start_offset_ms: 0,
                    end_offset_ms: 10000,
                    track_ids_json: "[]".into(),
                    created_at_ms: 1,
                    reason: None,
                },
            )?;
            Ok(())
        })
        .unwrap();
    let redacted = list(Default::default(), &storage).unwrap();
    assert_eq!(
        redacted
            .groups
            .iter()
            .find(|group| group.recording_id == first.recording)
            .unwrap()
            .index_coverage
            .total,
        0
    );
}

#[test]
fn details_cursor_reaches_every_section_beyond_the_legacy_limit() {
    let storage = fixture();
    storage.with_connection(|connection|{
        recording(connection,"a");job(connection,"a","audio",1,"audio","succeeded");
        for index in 0..520 {
            connection.execute("INSERT INTO processing_chunks(id,job_id,recording_id,source_kind,status,progress,attempt_count,created_at_ms,updated_at_ms) VALUES(?1,?2,?3,'audio','succeeded',1,1,1,1)",params![fixture_id(&format!("synthetic-{index:04}")),fixture_id("audio"),fixture_id("a")])?;
        }Ok(())
    }).unwrap();
    let mut cursor = None;
    let mut seen = BTreeSet::new();
    loop {
        let page = super::super::group_details::get(
            GetRecordingProcessingLaneInput {
                recording_id: fixture_id("a"),
                source_kind: "audio".into(),
                limit: Some(100),
                cursor,
                history: false,
            },
            &storage,
        )
        .unwrap();
        for unit in page.units {
            assert!(seen.insert(unit.unit.id));
        }
        cursor = page.next_cursor;
        if cursor.is_none() {
            break;
        }
    }
    assert_eq!(seen.len(), 521);
}
