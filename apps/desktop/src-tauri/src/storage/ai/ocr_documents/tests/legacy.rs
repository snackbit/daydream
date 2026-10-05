use super::{super::*, fixtures::*};
use crate::{ai::GetOcrDocumentInput, storage};
use uuid::Uuid;

#[test]
fn migration_reconstructs_legacy_geometry_without_claiming_continuous_coverage_or_hierarchy() {
    let (mut connection, chunk) = fixture_at(Some(19));
    for (timestamp, x, text) in [(0, 30, "world"), (0, 0, "Hello"), (1_000, 0, "Hello")] {
        storage::create_ocr_region(
            &connection,
            &storage::CreateVisualRegionInput {
                chunk_id: chunk.id.clone(),
                confidence: Some(0.4),
                frame_timestamp_ms: timestamp,
                id: Uuid::now_v7().to_string(),
                job_id: chunk.job_id.clone(),
                model: Some("legacy".into()),
                observation_kind: None,
                prompt_version: None,
                provider: "tesseract".into(),
                recording_id: chunk.recording_id.clone(),
                region_height: 10,
                region_width: 20,
                region_x: x,
                region_y: 0,
                source_frame_ref: format!("legacy-frame-{timestamp}"),
                text: text.into(),
            },
        )
        .unwrap();
    }
    crate::storage::schema::apply_migrations(&mut connection).unwrap();
    let page = history(&connection, &chunk);
    assert_eq!(page.items.len(), 2);
    let entry = &page.items[1];
    assert_eq!(entry.text, "Hello world");
    assert_eq!(entry.provenance, "reconstructed");
    assert_eq!(entry.width, None);
    assert_eq!(entry.occurrence_count, 1);
    assert_eq!(entry.observed_start_ms, entry.observed_end_ms);
    let document = get_ocr_document(
        &connection,
        &GetOcrDocumentInput {
            recording_id: chunk.recording_id.clone(),
            source_track_id: "video-0".into(),
            timestamp_ms: 0,
            document_id: None,
        },
    )
    .unwrap()
    .document
    .unwrap();
    assert!(document.blocks[0]
        .words
        .iter()
        .all(|word| word.hierarchy.is_none()));
    assert_eq!(document.blocks[0].words[0].confidence, Some(0.4));
    assert!(get_ocr_document(
        &connection,
        &GetOcrDocumentInput {
            recording_id: chunk.recording_id.clone(),
            source_track_id: "video-0".into(),
            timestamp_ms: 500,
            document_id: None
        }
    )
    .unwrap()
    .document
    .is_none());
}
