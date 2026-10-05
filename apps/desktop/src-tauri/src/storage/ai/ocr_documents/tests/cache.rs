use super::{super::*, fixtures::*};
use crate::storage;
use crate::{
    ai::{GetOcrDocumentInput, VisualRegion},
    storage::{OcrBlockInput, OcrWordInput},
};
use uuid::Uuid;

pub(super) fn word_frame(
    chunk: &storage::ProcessingChunkRow,
    timestamp: i64,
) -> storage::OcrFrameInput {
    let mut input = frame(timestamp, "Example");
    let raw_id = Uuid::now_v7().to_string();
    input.content = Some(storage::OcrContentInput {
        text: "Example".into(),
        blocks: vec![OcrBlockInput {
            id: "block".into(),
            order: 0,
            text: "Example".into(),
            confidence: None,
            region: None,
            raw_region_ids: vec![raw_id.clone()],
            words: vec![OcrWordInput {
                id: "word".into(),
                hierarchy: None,
                text: "Example".into(),
                confidence: Some(0.4),
                region: Some(VisualRegion {
                    x: 0,
                    y: 0,
                    width: 30,
                    height: 10,
                }),
                raw_region_id: Some(raw_id.clone()),
            }],
        }],
    });
    input.raw_regions = vec![storage::CreateVisualRegionInput {
        chunk_id: chunk.id.clone(),
        confidence: Some(0.4),
        frame_timestamp_ms: timestamp,
        id: raw_id,
        job_id: chunk.job_id.clone(),
        model: Some("eng".into()),
        observation_kind: None,
        prompt_version: None,
        provider: "tesseract".into(),
        recording_id: chunk.recording_id.clone(),
        region_height: 10,
        region_width: 30,
        region_x: 0,
        region_y: 0,
        source_frame_ref: input.source_frame_ref.clone(),
        text: "Example".into(),
    }];
    input
}

#[test]
fn cached_unchanged_frame_keeps_its_own_timestamp_raw_links_and_success_evidence() {
    let (mut connection, chunk) = fixture();
    let first = word_frame(&chunk, 0);
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
    let mut second = word_frame(&chunk, 1_000);
    second.pixel_hash = first.pixel_hash.clone();
    second.cache_content_id = Some(cached.content_id.clone());
    second.content = Some(cached.content);
    let raw_id = second.raw_regions[0].id.clone();
    save_ocr_frame_result(&mut connection, &chunk, &second).unwrap();
    let doc = get_ocr_document(
        &connection,
        &GetOcrDocumentInput {
            recording_id: chunk.recording_id.clone(),
            source_track_id: "video-0".into(),
            timestamp_ms: 1_000,
            document_id: None,
        },
    )
    .unwrap()
    .document
    .unwrap();
    assert_eq!(doc.entry.frame_timestamp_ms, 1_000);
    assert_eq!(doc.entry.source_frame_ref, second.source_frame_ref);
    assert_eq!(doc.content_id, Some(cached.content_id));
    assert_eq!(doc.blocks[0].words[0].raw_region_id, Some(raw_id));
    assert_eq!(doc.entry.support_timestamps_ms, [0, 1_000]);
    assert!(doc.entry.cache_hit);
    assert_eq!(
        storage::list_ocr_regions(&connection, &chunk.recording_id)
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn hidden_cache_cannot_be_reused_at_commit_and_fresh_recognition_keeps_current_provenance() {
    let (mut connection, old_chunk) = fixture();
    let old = word_frame(&old_chunk, 0);
    save_ocr_frame_result(&mut connection, &old_chunk, &old).unwrap();
    let cached = load_cached_ocr_document(
        &connection,
        &old_chunk.recording_id,
        "video-0",
        &old.pixel_hash,
        100,
        100,
        &old.config_fingerprint,
    )
    .unwrap()
    .unwrap();
    storage::create_redaction(
        &connection,
        &storage::CreateRedactionInput {
            created_at_ms: 1,
            end_offset_ms: 1_000,
            id: Uuid::now_v7().to_string(),
            reason: None,
            recording_id: old_chunk.recording_id.clone(),
            start_offset_ms: 0,
            track_ids_json: "[\"video-0\"]".into(),
        },
    )
    .unwrap();
    storage::update_processing_job_status(&connection, &old_chunk.job_id, "cancelled", None, 1)
        .unwrap();
    let job_id = Uuid::now_v7().to_string();
    storage::create_processing_job(
        &connection,
        &storage::CreateProcessingJobInput {
            created_at_ms: 10,
            error: None,
            id: job_id.clone(),
            kind: "ocr".into(),
            recording_id: old_chunk.recording_id.clone(),
            status: "running".into(),
            updated_at_ms: 10,
        },
    )
    .unwrap();
    let chunk = storage::create_processing_chunk(
        &connection,
        &storage::CreateProcessingChunkInput {
            attempt_count: 1,
            created_at_ms: 10,
            end_offset_ms: Some(20_000),
            error_code: None,
            id: Uuid::now_v7().to_string(),
            job_id,
            progress: 0.,
            recording_id: old_chunk.recording_id.clone(),
            result_code: None,
            segment_id: old_chunk.segment_id.clone(),
            source_kind: "ocr".into(),
            start_offset_ms: Some(1_000),
            status: "running".into(),
            track_id: None,
            updated_at_ms: 10,
        },
    )
    .unwrap();
    let mut fresh = word_frame(&chunk, 1_000);
    fresh.pixel_hash = old.pixel_hash;
    fresh.cache_content_id = Some(cached.content_id.clone());
    assert!(
        save_ocr_frame_result(&mut connection, &chunk, &fresh).is_err(),
        "an obsolete cache handle cannot bypass current privacy eligibility"
    );
    fresh.cache_content_id = None;
    let content = fresh.content.as_mut().unwrap();
    content.text = "Fresh recognition".into();
    content.blocks[0].text = "Fresh recognition".into();
    content.blocks[0].words[0].text = "Fresh recognition".into();
    fresh.raw_regions[0].text = "Fresh recognition".into();
    let raw_id = fresh.raw_regions[0].id.clone();
    save_ocr_frame_result(&mut connection, &chunk, &fresh).unwrap();
    let document = get_ocr_document(
        &connection,
        &GetOcrDocumentInput {
            recording_id: chunk.recording_id.clone(),
            source_track_id: "video-0".into(),
            timestamp_ms: 1_000,
            document_id: None,
        },
    )
    .unwrap()
    .document
    .unwrap();
    assert_ne!(document.content_id, Some(cached.content_id));
    assert_eq!(document.entry.text, "Fresh recognition");
    assert_eq!(document.entry.frame_timestamp_ms, 1_000);
    assert_eq!(document.entry.source_frame_ref, fresh.source_frame_ref);
    assert_eq!(document.entry.support_timestamps_ms, [1_000]);
    assert!(!document.entry.cache_hit);
    assert_eq!(document.blocks[0].words[0].raw_region_id, Some(raw_id));
    assert!(get_ocr_document(
        &connection,
        &GetOcrDocumentInput {
            recording_id: chunk.recording_id,
            source_track_id: "video-0".into(),
            timestamp_ms: 0,
            document_id: Some(old.id)
        }
    )
    .unwrap()
    .document
    .is_none());
}

#[test]
fn cache_requires_exact_pixels_dimensions_configuration_source_recording_and_visible_support() {
    let (mut connection, chunk) = fixture();
    let input = frame(0, "Example 17");
    save_ocr_frame_result(&mut connection, &chunk, &input).unwrap();
    assert!(load_cached_ocr_document(
        &connection,
        &chunk.recording_id,
        "video-0",
        &input.pixel_hash,
        100,
        100,
        &input.config_fingerprint
    )
    .unwrap()
    .is_some());
    for (recording, source, pixel, width, config) in [
        (
            "other",
            "video-0",
            input.pixel_hash.as_str(),
            100,
            input.config_fingerprint.as_str(),
        ),
        (
            chunk.recording_id.as_str(),
            "video-1",
            input.pixel_hash.as_str(),
            100,
            input.config_fingerprint.as_str(),
        ),
        (
            chunk.recording_id.as_str(),
            "video-0",
            "different",
            100,
            input.config_fingerprint.as_str(),
        ),
        (
            chunk.recording_id.as_str(),
            "video-0",
            input.pixel_hash.as_str(),
            99,
            input.config_fingerprint.as_str(),
        ),
        (
            chunk.recording_id.as_str(),
            "video-0",
            input.pixel_hash.as_str(),
            100,
            "different",
        ),
    ] {
        assert!(load_cached_ocr_document(
            &connection,
            recording,
            source,
            pixel,
            width,
            100,
            config
        )
        .unwrap()
        .is_none());
    }
    storage::create_redaction(
        &connection,
        &storage::CreateRedactionInput {
            created_at_ms: 1,
            end_offset_ms: 500,
            id: Uuid::now_v7().to_string(),
            reason: None,
            recording_id: chunk.recording_id.clone(),
            start_offset_ms: 0,
            track_ids_json: "[\"video-0\"]".into(),
        },
    )
    .unwrap();
    assert!(load_cached_ocr_document(
        &connection,
        &chunk.recording_id,
        "video-0",
        &input.pixel_hash,
        100,
        100,
        &input.config_fingerprint
    )
    .unwrap()
    .is_none());
    assert!(history(&connection, &chunk).items.is_empty());
}
