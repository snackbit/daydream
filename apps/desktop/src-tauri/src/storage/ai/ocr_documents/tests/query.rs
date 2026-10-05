use super::{super::*, fixtures::*};
use crate::{
    ai::{GetOcrDocumentInput, GetOcrHistoryInput},
    storage,
};
use uuid::Uuid;

#[test]
fn redaction_of_a_support_sample_splits_run_and_unhide_recomputes_its_supports() {
    let (mut connection, chunk) = fixture();
    for timestamp in [0, 1_000, 2_000] {
        save_ocr_frame_result(
            &mut connection,
            &chunk,
            &frame(timestamp, "Same literal 42"),
        )
        .unwrap();
    }
    assert_eq!(
        history(&connection, &chunk).items[0].support_timestamps_ms,
        [0, 1_000, 2_000]
    );
    let id = Uuid::now_v7().to_string();
    storage::create_redaction(
        &connection,
        &storage::CreateRedactionInput {
            created_at_ms: 1,
            end_offset_ms: 2_000,
            id: id.clone(),
            reason: None,
            recording_id: chunk.recording_id.clone(),
            start_offset_ms: 1_000,
            track_ids_json: "[\"video-0\"]".into(),
        },
    )
    .unwrap();
    let cut = history(&connection, &chunk);
    assert_eq!(cut.items.len(), 2);
    assert_eq!(cut.items[0].support_timestamps_ms, [2_000]);
    assert_eq!(cut.items[1].support_timestamps_ms, [0]);
    storage::delete_redaction(&connection, &chunk.recording_id, &id).unwrap();
    assert_eq!(
        history(&connection, &chunk).items[0].support_timestamps_ms,
        [0, 1_000, 2_000]
    );
}

#[test]
fn source_configuration_change_and_missing_sample_never_merge_equal_text() {
    let (mut connection, chunk) = fixture();
    save_ocr_frame_result(&mut connection, &chunk, &frame(0, "literal 42")).unwrap();
    let mut changed = frame(1_000, "literal 42");
    changed.engine = "vision".into();
    changed.provider = "openai-compatible".into();
    changed.config_fingerprint = "vision-new-prompt".into();
    save_ocr_frame_result(&mut connection, &chunk, &changed).unwrap();
    let mut gap = frame(3_000, "literal 42");
    gap.config_fingerprint = changed.config_fingerprint;
    gap.engine = changed.engine;
    gap.provider = changed.provider;
    save_ocr_frame_result(&mut connection, &chunk, &gap).unwrap();
    assert_eq!(history(&connection, &chunk).items.len(), 3);
}

#[test]
fn adjacent_literal_documents_coalesce_whitespace_only_and_empty_failure_gap_split() {
    let (mut connection, chunk) = fixture();
    for (timestamp, text) in [
        (0, "Total: 42"),
        (1_000, "Total:   42"),
        (2_000, "total: 42"),
        (3_000, ""),
        (4_000, ""),
    ] {
        assert!(save_ocr_frame_result(&mut connection, &chunk, &frame(timestamp, text)).unwrap());
    }
    let mut failed = frame(5_000, "");
    failed.status = "failed".into();
    failed.content = None;
    failed.error_code = Some("synthetic_failure".into());
    save_ocr_frame_result(&mut connection, &chunk, &failed).unwrap();
    save_ocr_frame_result(&mut connection, &chunk, &frame(7_000, "Total: 42")).unwrap();
    let page = history(&connection, &chunk);
    assert_eq!(page.items.len(), 6);
    let initial = page.items.last().unwrap();
    assert_eq!(initial.support_timestamps_ms, [0, 1_000]);
    assert_eq!(initial.observed_end_ms, 1_000);
    assert_eq!(initial.occurrence_count, 2);
    assert_eq!(page.items[1].status, "failed");
    assert_eq!(page.items[2].text, "");
    assert_eq!(page.items[3].text, "");
}

#[test]
fn paginated_history_and_selected_document_remain_source_scoped_after_redaction_changes() {
    let (mut connection, chunk) = fixture();
    let mut ids = vec![];
    for timestamp in [0, 1_000, 2_000, 3_000] {
        let input = frame(timestamp, &format!("revision {timestamp}"));
        ids.push(input.id.clone());
        save_ocr_frame_result(&mut connection, &chunk, &input).unwrap();
    }
    let first = get_ocr_history(
        &connection,
        &GetOcrHistoryInput {
            recording_id: chunk.recording_id.clone(),
            source_track_id: None,
            limit: Some(2),
            cursor: None,
        },
    )
    .unwrap();
    assert_eq!(first.items.len(), 2);
    let cursor = first.next_cursor.unwrap();
    assert_eq!(cursor.timestamp_ms, 2_000);
    storage::create_redaction(
        &connection,
        &storage::CreateRedactionInput {
            created_at_ms: 1,
            end_offset_ms: 2_000,
            id: Uuid::now_v7().to_string(),
            reason: None,
            recording_id: chunk.recording_id.clone(),
            start_offset_ms: 1_000,
            track_ids_json: "[\"video-0\"]".into(),
        },
    )
    .unwrap();
    let second = get_ocr_history(
        &connection,
        &GetOcrHistoryInput {
            recording_id: chunk.recording_id.clone(),
            source_track_id: None,
            limit: Some(2),
            cursor: Some(cursor),
        },
    )
    .unwrap();
    assert_eq!(second.items.len(), 1);
    assert_eq!(second.items[0].frame_timestamp_ms, 0);
    for (source, id, timestamp) in [
        ("video-0", Some(ids[1].clone()), 1_000),
        ("video-1", Some(ids[0].clone()), 0),
    ] {
        assert!(get_ocr_document(
            &connection,
            &GetOcrDocumentInput {
                recording_id: chunk.recording_id.clone(),
                source_track_id: source.into(),
                timestamp_ms: timestamp,
                document_id: id
            }
        )
        .unwrap()
        .document
        .is_none());
    }
}

#[test]
fn newer_failed_ocr_job_does_not_present_previous_document_as_current() {
    let (mut connection, chunk) = fixture();
    save_ocr_frame_result(&mut connection, &chunk, &frame(0, "old")).unwrap();
    let job_id = Uuid::now_v7().to_string();
    storage::create_processing_job(
        &connection,
        &storage::CreateProcessingJobInput {
            created_at_ms: 10,
            error: Some("synthetic_failure".into()),
            id: job_id.clone(),
            kind: "ocr".into(),
            recording_id: chunk.recording_id.clone(),
            status: "failed".into(),
            updated_at_ms: 10,
        },
    )
    .unwrap();
    storage::create_processing_chunk(
        &connection,
        &storage::CreateProcessingChunkInput {
            attempt_count: 1,
            created_at_ms: 10,
            end_offset_ms: chunk.end_offset_ms,
            error_code: Some("synthetic_failure".into()),
            id: Uuid::now_v7().to_string(),
            job_id,
            progress: 0.,
            recording_id: chunk.recording_id.clone(),
            result_code: None,
            segment_id: chunk.segment_id.clone(),
            source_kind: "ocr".into(),
            start_offset_ms: chunk.start_offset_ms,
            status: "failed".into(),
            track_id: None,
            updated_at_ms: 10,
        },
    )
    .unwrap();
    assert!(history(&connection, &chunk).items.is_empty());
    let selection = get_ocr_document(
        &connection,
        &GetOcrDocumentInput {
            recording_id: chunk.recording_id,
            source_track_id: "video-0".into(),
            timestamp_ms: 0,
            document_id: None,
        },
    )
    .unwrap();
    assert!(selection.document.is_none());
    let job = selection
        .job
        .expect("a pre-frame failure still exposes its actual OCR job");
    assert_eq!(job.status, "failed");
    assert_eq!(job.error_code.as_deref(), Some("synthetic_failure"));
}
