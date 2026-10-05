use super::{super::*, fixtures::*};
use crate::{
    ai::{GetOcrHistoryInput, GetOcrTimelineFramesInput},
    storage,
};
use uuid::Uuid;

fn input(recording_id: &str) -> GetOcrTimelineFramesInput {
    GetOcrTimelineFramesInput {
        recording_id: recording_id.into(),
        source_track_id: None,
        start_timestamp_ms: None,
        end_timestamp_ms: None,
        limit: None,
        cursor: None,
    }
}

#[test]
fn timeline_index_preserves_every_actual_sample_when_text_and_pixels_are_unchanged() {
    let (mut connection, chunk) = fixture();
    let first = frame(0, "Same observed text");
    save_ocr_frame_result(&mut connection, &chunk, &first).unwrap();
    let cached = load_cached_ocr_document(
        &connection,
        &chunk.recording_id,
        "video-0",
        &first.pixel_hash,
        100,
        100,
        &first.config_fingerprint,
    )
    .unwrap()
    .unwrap();
    for timestamp in [5_000, 10_000] {
        let mut next = frame(timestamp, "Same observed text");
        next.pixel_hash = first.pixel_hash.clone();
        next.cache_content_id = Some(cached.content_id.clone());
        next.content = Some(cached.content.clone());
        save_ocr_frame_result(&mut connection, &chunk, &next).unwrap();
    }
    // Recognition cadence is metadata, independent of text-history coalescing.
    connection
        .execute(
            "UPDATE ocr_frame_occurrences SET sample_end_ms=frame_timestamp_ms+5000",
            [],
        )
        .unwrap();
    assert_eq!(
        get_ocr_history(
            &connection,
            &GetOcrHistoryInput {
                recording_id: chunk.recording_id.clone(),
                source_track_id: None,
                limit: None,
                cursor: None
            }
        )
        .unwrap()
        .items
        .len(),
        1
    );
    let page = get_ocr_timeline_frames(&connection, &input(&chunk.recording_id)).unwrap();
    assert_eq!(
        page.frames
            .iter()
            .map(|frame| frame.frame_timestamp_ms)
            .collect::<Vec<_>>(),
        [0, 5_000, 10_000]
    );
    let ids = page
        .frames
        .iter()
        .map(|frame| frame.id.as_str())
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(ids.len(), 3);
    for sample in &page.frames {
        assert_eq!(sample.document_id, sample.id);
        assert_eq!(sample.source_track_id, "video-0");
        assert_eq!(sample.ocr_output_count, 1);
        assert_eq!(sample.text_empty, Some(false));
        assert_eq!(sample.sample_end_ms, sample.frame_timestamp_ms + 5_000);
    }
    let json = serde_json::to_value(&page).unwrap();
    assert!(json["frames"][0].get("text").is_none());
    assert!(json["frames"][0].get("blocks").is_none());
}

#[test]
fn timeline_index_reconstructs_legacy_late_samples_without_a_coarse_zero_marker() {
    let (mut connection, chunk) = fixture_at(Some(19));
    connection
        .execute(
            "UPDATE recordings SET ended_at_ms=40000 WHERE id=?1",
            [&chunk.recording_id],
        )
        .unwrap();
    connection.execute("UPDATE recording_segments SET duration_ms=40000,ended_at_ms=40000 WHERE recording_id=?1",[&chunk.recording_id]).unwrap();
    connection
        .execute(
            "UPDATE processing_chunks SET end_offset_ms=40000 WHERE id=?1",
            [&chunk.id],
        )
        .unwrap();
    for timestamp in [33_000, 36_000, 38_000] {
        storage::create_ocr_region(
            &connection,
            &storage::CreateVisualRegionInput {
                chunk_id: chunk.id.clone(),
                confidence: Some(0.5),
                frame_timestamp_ms: timestamp,
                id: Uuid::now_v7().to_string(),
                job_id: chunk.job_id.clone(),
                model: None,
                observation_kind: None,
                prompt_version: None,
                provider: "tesseract".into(),
                recording_id: chunk.recording_id.clone(),
                region_height: 10,
                region_width: 20,
                region_x: 0,
                region_y: 0,
                source_frame_ref: format!("actual-legacy-sample-{timestamp}"),
                text: "Repeated legacy text".into(),
            },
        )
        .unwrap();
    }
    crate::storage::schema::apply_migrations(&mut connection).unwrap();
    let page = get_ocr_timeline_frames(&connection, &input(&chunk.recording_id)).unwrap();
    assert_eq!(
        page.frames
            .iter()
            .map(|frame| frame.frame_timestamp_ms)
            .collect::<Vec<_>>(),
        [33_000, 36_000, 38_000]
    );
    assert_eq!(
        page.frames[0].source_frame_ref,
        "actual-legacy-sample-33000"
    );
    assert!(page
        .frames
        .iter()
        .all(|frame| frame.sample_end_ms == frame.frame_timestamp_ms + 1));
}

#[test]
fn timeline_index_distinguishes_empty_success_failure_and_unprocessed_gap_without_inventing_text() {
    let (mut connection, chunk) = fixture();
    save_ocr_frame_result(&mut connection, &chunk, &frame(0, "")).unwrap();
    let mut failed = frame(1_000, "");
    failed.status = "failed".into();
    failed.content = None;
    failed.error_code = Some("synthetic_failed_frame".into());
    save_ocr_frame_result(&mut connection, &chunk, &failed).unwrap();
    let mut gap = frame(2_000, "");
    gap.status = "gap".into();
    gap.content = None;
    save_ocr_frame_result(&mut connection, &chunk, &gap).unwrap();
    let frames = get_ocr_timeline_frames(&connection, &input(&chunk.recording_id))
        .unwrap()
        .frames;
    assert_eq!(frames.len(), 3);
    assert_eq!(frames[0].status, "succeeded");
    assert_eq!(frames[0].text_empty, Some(true));
    assert_eq!(frames[0].ocr_output_count, 1);
    assert_eq!(frames[1].status, "failed");
    assert_eq!(
        frames[1].error_code.as_deref(),
        Some("synthetic_failed_frame")
    );
    assert_eq!(frames[1].text_empty, None);
    assert_eq!(frames[1].ocr_output_count, 0);
    assert_eq!(frames[2].status, "gap");
    assert_eq!(frames[2].ocr_output_count, 0);
    assert_eq!(frames[2].text_empty, None);
}

#[test]
fn timeline_index_pagination_ranges_and_privacy_filter_actual_samples_before_return() {
    let (mut connection, chunk) = fixture();
    for timestamp in [0, 5_000, 10_000] {
        save_ocr_frame_result(&mut connection, &chunk, &frame(timestamp, "same")).unwrap();
    }
    let mut request = input(&chunk.recording_id);
    request.limit = Some(1);
    let first = get_ocr_timeline_frames(&connection, &request).unwrap();
    assert_eq!(first.frames.len(), 1);
    request.cursor = first.next_cursor;
    let second = get_ocr_timeline_frames(&connection, &request).unwrap();
    assert_eq!(second.frames[0].frame_timestamp_ms, 5_000);
    storage::create_redaction(
        &connection,
        &storage::CreateRedactionInput {
            created_at_ms: 1,
            end_offset_ms: 6_000,
            id: Uuid::now_v7().to_string(),
            reason: None,
            recording_id: chunk.recording_id.clone(),
            start_offset_ms: 5_000,
            track_ids_json: "[\"video-0\"]".into(),
        },
    )
    .unwrap();
    request.cursor = None;
    request.limit = Some(5_000);
    let visible = get_ocr_timeline_frames(&connection, &request).unwrap();
    assert_eq!(
        visible
            .frames
            .iter()
            .map(|frame| frame.frame_timestamp_ms)
            .collect::<Vec<_>>(),
        [0, 10_000]
    );
    request.start_timestamp_ms = Some(8_000);
    request.end_timestamp_ms = Some(12_000);
    assert_eq!(
        get_ocr_timeline_frames(&connection, &request)
            .unwrap()
            .frames
            .len(),
        1
    );
    request.source_track_id = Some("video-1".into());
    assert!(get_ocr_timeline_frames(&connection, &request)
        .unwrap()
        .frames
        .is_empty());
}
