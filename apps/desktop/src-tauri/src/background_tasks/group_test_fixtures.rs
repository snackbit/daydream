use crate::storage::{self, AppStorage};
use rusqlite::Connection;

pub(super) fn fixture_id(label: &str) -> String {
    let value = label.bytes().fold(0u128, |value, byte| {
        value.wrapping_mul(131).wrapping_add(u128::from(byte))
    });
    uuid::Uuid::from_u128(value).to_string()
}

pub(super) fn recording(connection: &Connection, id: &str) {
    storage::create_recording(
        connection,
        &storage::CreateRecordingInput {
            id: fixture_id(id),
            active_scene_id: None,
            created_at_ms: 1,
            default_fps: 1.0,
            notes: None,
            output_root: None,
            quality_label: "balanced-ocr".into(),
            storage_dir: "/tmp/synthetic-processing-group".into(),
        },
    )
    .unwrap();
    connection
        .execute(
            "UPDATE recordings SET status='completed',title='Synthetic recording' WHERE id=?1",
            [fixture_id(id)],
        )
        .unwrap();
}
pub(super) fn job(
    connection: &Connection,
    recording: &str,
    id: &str,
    time: i64,
    kind: &str,
    status: &str,
) {
    storage::create_processing_job(
        connection,
        &storage::CreateProcessingJobInput {
            id: fixture_id(id),
            recording_id: fixture_id(recording),
            kind: "ai-processing".into(),
            status: status.into(),
            created_at_ms: time,
            updated_at_ms: time,
            error: None,
        },
    )
    .unwrap();
    storage::create_processing_chunk(
        connection,
        &storage::CreateProcessingChunkInput {
            id: fixture_id(&format!("{id}-chunk")),
            job_id: fixture_id(id),
            recording_id: fixture_id(recording),
            source_kind: kind.into(),
            segment_id: None,
            track_id: None,
            start_offset_ms: Some(0),
            end_offset_ms: Some(1000),
            attempt_count: 1,
            created_at_ms: time,
            updated_at_ms: time,
            status: status.into(),
            progress: if status == "queued" { 0.0 } else { 1.0 },
            error_code: if status == "failed" {
                Some("ai_ocr_provider_failed".into())
            } else {
                None
            },
            result_code: None,
        },
    )
    .unwrap();
}
pub(super) fn fixture() -> AppStorage {
    storage::test_storage(std::env::temp_dir().join(format!(
        "synthetic-processing-groups-{}",
        uuid::Uuid::now_v7()
    )))
}
