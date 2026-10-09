use super::{super::*, fixtures::*};
use crate::storage;

#[test]
fn uuid_video_sources_persist_only_the_selected_eligible_source() {
    let (mut connection, mut chunk) = fixture();
    let sources = (0..2)
        .map(|index| storage::RecordingVideoSource {
            track_id: uuid::Uuid::now_v7().to_string(),
            source_id: format!("synthetic-{index}"),
            label: format!("Synthetic {index}"),
            width: 100,
            height: 100,
            config_json: "{}".into(),
        })
        .collect::<Vec<_>>();
    storage::save_recording_video_sources(&connection, &chunk.recording_id, &sources, 0).unwrap();
    storage::save_segment_video_streams(
        &connection,
        chunk.segment_id.as_ref().unwrap(),
        &sources,
        0.2,
    )
    .unwrap();
    for source in &sources {
        connection
            .execute(
                "UPDATE processing_chunks SET track_id=?1 WHERE id=?2",
                rusqlite::params![source.track_id, chunk.id],
            )
            .unwrap();
        chunk.track_id = Some(source.track_id.clone());
        let mut input = frame(0, "synthetic source text");
        input.source_track_id = source.track_id.clone();
        assert!(save_ocr_frame_result(&mut connection, &chunk, &input).unwrap());
        input.source_track_id = "video-0".into();
        assert!(save_ocr_frame_result(&mut connection, &chunk, &input).is_err());
    }
    let mut input = frame(0, "wrong source");
    input.source_track_id = sources[0].track_id.clone();
    assert!(save_ocr_frame_result(&mut connection, &chunk, &input).is_err());
    connection
        .execute(
            "UPDATE processing_chunks SET track_id=NULL WHERE id=?1",
            [&chunk.id],
        )
        .unwrap();
    chunk.track_id = None;
    assert!(save_ocr_frame_result(&mut connection, &chunk, &input).is_err());
    // UUID sources retain the same attempt, cancellation and redaction fences
    // as the legacy composite. A rejected result cannot create an occurrence.
    connection
        .execute(
            "UPDATE processing_chunks SET track_id=?1 WHERE id=?2",
            rusqlite::params![sources[0].track_id, chunk.id],
        )
        .unwrap();
    chunk.track_id = Some(sources[0].track_id.clone());
    input.frame_timestamp_ms = 2000;
    input.sample_end_ms = 3000;
    connection
        .execute(
            "UPDATE processing_chunks SET attempt_count=attempt_count+1 WHERE id=?1",
            [&chunk.id],
        )
        .unwrap();
    assert!(!save_ocr_frame_result(&mut connection, &chunk, &input).unwrap());
    chunk = storage::get_processing_chunk(&connection, &chunk.id)
        .unwrap()
        .unwrap();
    storage::update_processing_job_status(&connection, &chunk.job_id, "cancelled", None, 1)
        .unwrap();
    assert!(!save_ocr_frame_result(&mut connection, &chunk, &input).unwrap());
    storage::update_processing_job_status(&connection, &chunk.job_id, "running", None, 2).unwrap();
    let redaction_id = uuid::Uuid::now_v7().to_string();
    storage::create_redaction(
        &connection,
        &storage::CreateRedactionInput {
            id: redaction_id.clone(),
            recording_id: chunk.recording_id.clone(),
            start_offset_ms: 2000,
            end_offset_ms: 3000,
            created_at_ms: 3,
            reason: None,
            track_ids_json: serde_json::to_string(&[&sources[0].track_id]).unwrap(),
        },
    )
    .unwrap();
    assert!(!save_ocr_frame_result(&mut connection, &chunk, &input).unwrap());
    let count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM ocr_frame_occurrences WHERE frame_timestamp_ms=2000",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 0);
    storage::delete_redaction(&connection, &chunk.recording_id, &redaction_id).unwrap();
    assert!(save_ocr_frame_result(&mut connection, &chunk, &input).unwrap());
}

#[test]
fn durable_ocr_schema_tracks_successful_empty_frames_and_pinned_configs() {
    let storage =
        crate::storage::test_storage(std::env::temp_dir().join("daydream-ocr-test.sqlite"));
    storage
        .with_connection(|connection| {
            for table in [
                "ocr_document_contents",
                "ocr_frame_occurrences",
                "processing_job_ocr_configs",
            ] {
                let exists: bool = connection.query_row(
                    "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
                    [table],
                    |row| row.get(0),
                )?;
                assert!(exists, "durable OCR table {table} must exist");
            }
            Ok(())
        })
        .unwrap();
}

#[test]
fn stale_attempt_and_cancelled_job_cannot_persist_or_expose_document_content() {
    let (mut connection, chunk) = fixture();
    connection
        .execute(
            "UPDATE processing_chunks SET attempt_count=attempt_count+1 WHERE id=?1",
            [&chunk.id],
        )
        .unwrap();
    assert!(!save_ocr_frame_result(&mut connection, &chunk, &frame(0, "stale")).unwrap());
    let current = storage::get_processing_chunk(&connection, &chunk.id)
        .unwrap()
        .unwrap();
    storage::update_processing_job_status(&connection, &chunk.job_id, "cancelled", None, 1)
        .unwrap();
    assert!(!save_ocr_frame_result(&mut connection, &current, &frame(0, "cancelled")).unwrap());
    assert!(history(&connection, &current).items.is_empty());
}

#[test]
fn failed_occurrence_insert_rolls_back_content_and_pinned_configuration_is_immutable() {
    let (mut connection, chunk) = fixture();
    save_processing_job_ocr_config(&connection, &chunk.job_id, "{\"engine\":\"tesseract\"}")
        .unwrap();
    save_processing_job_ocr_config(&connection, &chunk.job_id, "{\"engine\":\"vision\"}").unwrap();
    assert_eq!(
        get_processing_job_ocr_config(&connection, &chunk.job_id)
            .unwrap()
            .unwrap(),
        "{\"engine\":\"tesseract\"}"
    );
    connection.execute_batch("CREATE TRIGGER fail_occurrence BEFORE INSERT ON ocr_frame_occurrences BEGIN SELECT RAISE(FAIL, 'synthetic failed commit'); END;").unwrap();
    assert!(save_ocr_frame_result(&mut connection, &chunk, &frame(0, "atomic")).is_err());
    let count: i64 = connection
        .query_row("SELECT COUNT(*) FROM ocr_document_contents", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(count, 0);
}

#[test]
fn deleting_recording_removes_documents_occurrences_and_config_cache() {
    let (mut connection, chunk) = fixture();
    save_ocr_frame_result(&mut connection, &chunk, &frame(0, "delete me")).unwrap();
    save_processing_job_ocr_config(&connection, &chunk.job_id, "{}").unwrap();
    storage::delete_recording(&connection, &chunk.recording_id).unwrap();
    for table in [
        "ocr_document_contents",
        "ocr_frame_occurrences",
        "processing_job_ocr_configs",
    ] {
        let count: i64 = connection
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 0);
    }
}
