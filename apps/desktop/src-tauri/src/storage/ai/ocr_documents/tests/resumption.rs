use super::{super::*, cache::word_frame, fixtures::*};
use crate::{ai::GetOcrDocumentInput, storage};
use uuid::Uuid;

#[test]
fn retry_hides_previous_attempt_but_retains_successful_cache_and_original_job_config() {
    let (mut connection, chunk) = fixture();
    save_ocr_frame_result(&mut connection, &chunk, &frame(0, "retry")).unwrap();
    save_processing_job_ocr_config(&connection, &chunk.job_id, "{\"engine\":\"tesseract\"}")
        .unwrap();
    connection
        .execute(
            "UPDATE processing_chunks SET status='failed' WHERE id=?1",
            [&chunk.id],
        )
        .unwrap();
    storage::retry_processing_job_chunks(&connection, &chunk.job_id, 1).unwrap();
    assert!(history(&connection, &chunk).items.is_empty());
    let count: i64 = connection
        .query_row("SELECT COUNT(*) FROM ocr_document_contents", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(count, 1);
    assert_eq!(
        get_processing_job_ocr_config(&connection, &chunk.job_id)
            .unwrap()
            .unwrap(),
        "{\"engine\":\"tesseract\"}"
    );
}

#[test]
fn resumed_and_retried_chunks_reuse_only_committed_success_with_fresh_attempt_provenance() {
    for pause in [true, false] {
        let (mut connection, old_chunk) = fixture();
        let first = word_frame(&old_chunk, 0);
        save_ocr_frame_result(&mut connection, &old_chunk, &first).unwrap();
        let mut failed = frame(1_000, "");
        failed.status = "failed".into();
        failed.content = None;
        failed.error_code = Some("synthetic_failure".into());
        save_ocr_frame_result(&mut connection, &old_chunk, &failed).unwrap();
        if pause {
            storage::cancel_processing_chunks(&connection, &old_chunk.job_id, &["running"], 1)
                .unwrap();
            storage::update_processing_job_status(
                &connection,
                &old_chunk.job_id,
                "paused",
                None,
                1,
            )
            .unwrap();
            storage::requeue_processing_chunks(&connection, &old_chunk.job_id, &["cancelled"], 2)
                .unwrap();
            storage::update_processing_job_status(
                &connection,
                &old_chunk.job_id,
                "queued",
                None,
                2,
            )
            .unwrap();
        } else {
            connection
                .execute(
                    "UPDATE processing_chunks SET status='failed' WHERE id=?1",
                    [&old_chunk.id],
                )
                .unwrap();
            storage::update_processing_job_status(
                &connection,
                &old_chunk.job_id,
                "failed",
                Some("synthetic_failure"),
                1,
            )
            .unwrap();
            storage::retry_processing_job_chunks(&connection, &old_chunk.job_id, 2).unwrap();
        }
        let pending = get_ocr_document(
            &connection,
            &GetOcrDocumentInput {
                recording_id: old_chunk.recording_id.clone(),
                source_track_id: "video-0".into(),
                timestamp_ms: 0,
                document_id: None,
            },
        )
        .unwrap();
        assert!(
            pending.document.is_none(),
            "old attempt output is not current Inspector evidence"
        );
        assert_eq!(pending.job.unwrap().status, "queued");
        let cached = load_cached_ocr_document(
            &connection,
            &old_chunk.recording_id,
            "video-0",
            &first.pixel_hash,
            100,
            100,
            &first.config_fingerprint,
        )
        .unwrap()
        .expect("committed success survives pause/retry");
        assert!(
            load_cached_ocr_document(
                &connection,
                &old_chunk.recording_id,
                "video-0",
                &failed.pixel_hash,
                100,
                100,
                &failed.config_fingerprint
            )
            .unwrap()
            .is_none(),
            "failed recognition never enters cache"
        );
        storage::update_processing_job_status(&connection, &old_chunk.job_id, "running", None, 3)
            .unwrap();
        let current = storage::claim_processing_chunk(&connection, &old_chunk.id, 0., 3)
            .unwrap()
            .unwrap();
        assert_eq!(current.attempt_count, old_chunk.attempt_count + 1);
        assert!(
            !save_ocr_frame_result(&mut connection, &old_chunk, &first).unwrap(),
            "a stale worker still cannot commit through cached content"
        );
        assert!(get_ocr_document(
            &connection,
            &GetOcrDocumentInput {
                recording_id: current.recording_id.clone(),
                source_track_id: "video-0".into(),
                timestamp_ms: 0,
                document_id: None
            }
        )
        .unwrap()
        .document
        .is_none());
        let mut reused = word_frame(&current, 0);
        reused.source_frame_ref = "current-attempt-frame-0".into();
        reused.raw_regions[0].source_frame_ref = reused.source_frame_ref.clone();
        reused.pixel_hash = first.pixel_hash.clone();
        reused.cache_content_id = Some(cached.content_id.clone());
        reused.content = Some(cached.content);
        let raw_id = reused.raw_regions[0].id.clone();
        save_ocr_frame_result(&mut connection, &current, &reused).unwrap();
        let document = get_ocr_document(
            &connection,
            &GetOcrDocumentInput {
                recording_id: current.recording_id.clone(),
                source_track_id: "video-0".into(),
                timestamp_ms: 0,
                document_id: None,
            },
        )
        .unwrap()
        .document
        .unwrap();
        assert_eq!(document.entry.id, reused.id);
        assert_eq!(document.entry.source_frame_ref, reused.source_frame_ref);
        assert_eq!(document.entry.support_timestamps_ms, [0]);
        assert_eq!(document.blocks[0].words[0].raw_region_id, Some(raw_id));
        assert!(document.entry.cache_hit);
        save_ocr_frame_result(&mut connection, &current, &word_frame(&current, 1_000)).unwrap();
        assert_eq!(
            history(&connection, &current).items[0].support_timestamps_ms,
            [0, 1_000]
        );
        storage::create_redaction(
            &connection,
            &storage::CreateRedactionInput {
                created_at_ms: 4,
                end_offset_ms: 1_000,
                id: Uuid::now_v7().to_string(),
                reason: None,
                recording_id: current.recording_id.clone(),
                start_offset_ms: 0,
                track_ids_json: "[\"video-0\"]".into(),
            },
        )
        .unwrap();
        assert!(
            load_cached_ocr_document(
                &connection,
                &current.recording_id,
                "video-0",
                &first.pixel_hash,
                100,
                100,
                &first.config_fingerprint
            )
            .unwrap()
            .is_none(),
            "current redaction excludes historical and fresh support"
        );
    }
}
