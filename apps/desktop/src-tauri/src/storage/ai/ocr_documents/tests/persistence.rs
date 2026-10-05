use super::{super::*, fixtures::*};
use crate::storage;

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
