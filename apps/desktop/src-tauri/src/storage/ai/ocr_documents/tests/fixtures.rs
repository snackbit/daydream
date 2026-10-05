use super::super::*;
use crate::{
    ai::GetOcrHistoryInput,
    storage::{self, ProcessingChunkRow},
};
use rusqlite::Connection;
use uuid::Uuid;

pub(super) fn fixture() -> (Connection, ProcessingChunkRow) {
    fixture_at(None)
}
pub(super) fn fixture_at(version: Option<usize>) -> (Connection, ProcessingChunkRow) {
    let mut connection = Connection::open_in_memory().unwrap();
    crate::storage::schema::enable_connection_pragmas(&connection).unwrap();
    match version {
        Some(version) => crate::storage::schema::migrations()
            .to_version(&mut connection, version)
            .unwrap(),
        None => crate::storage::schema::apply_migrations(&mut connection).unwrap(),
    };
    let recording_id = Uuid::now_v7().to_string();
    let segment_id = Uuid::now_v7().to_string();
    let job_id = Uuid::now_v7().to_string();
    let chunk_id = Uuid::now_v7().to_string();
    storage::create_recording(
        &connection,
        &storage::CreateRecordingInput {
            id: recording_id.clone(),
            active_scene_id: None,
            created_at_ms: 0,
            default_fps: 1.,
            notes: None,
            output_root: None,
            quality_label: "balanced-ocr".into(),
            storage_dir: "/tmp/synthetic-ocr".into(),
        },
    )
    .unwrap();
    storage::create_recording_segment(
        &connection,
        &storage::CreateRecordingSegmentInput {
            file_path: "/tmp/synthetic-ocr/segment.mkv".into(),
            id: segment_id.clone(),
            recording_id: recording_id.clone(),
            segment_index: 1,
            started_at_ms: 0,
        },
    )
    .unwrap();
    storage::finalize_recording_segment(
        &connection,
        &storage::FinalizeRecordingSegmentInput {
            desktop_audio_stream_index: None,
            duration_ms: 20_000,
            ended_at_ms: 20_000,
            id: segment_id.clone(),
            microphone_audio_stream_index: None,
            probe_json: None,
            updated_at_ms: 20_000,
            video_stream_index: Some(0),
        },
    )
    .unwrap();
    storage::mark_recording_started(&connection, &recording_id, 0).unwrap();
    storage::mark_recording_completed(&connection, &recording_id, 20_000).unwrap();
    storage::create_processing_job(
        &connection,
        &storage::CreateProcessingJobInput {
            created_at_ms: 0,
            error: None,
            id: job_id.clone(),
            kind: "ocr".into(),
            recording_id: recording_id.clone(),
            status: "running".into(),
            updated_at_ms: 0,
        },
    )
    .unwrap();
    let chunk = storage::create_processing_chunk(
        &connection,
        &storage::CreateProcessingChunkInput {
            attempt_count: 1,
            created_at_ms: 0,
            end_offset_ms: Some(20_000),
            error_code: None,
            id: chunk_id,
            job_id,
            progress: 0.,
            recording_id,
            result_code: None,
            segment_id: Some(segment_id),
            source_kind: "ocr".into(),
            start_offset_ms: Some(0),
            status: "running".into(),
            track_id: None,
            updated_at_ms: 0,
        },
    )
    .unwrap();
    (connection, chunk)
}
pub(super) fn frame(timestamp: i64, text: &str) -> OcrFrameInput {
    OcrFrameInput {
        id: Uuid::now_v7().to_string(),
        frame_timestamp_ms: timestamp,
        sample_end_ms: timestamp + 1_000,
        source_frame_ref: format!("synthetic-frame-{timestamp}"),
        source_track_id: "video-0".into(),
        width: 100,
        height: 100,
        config_fingerprint: "tesseract-options-v1".into(),
        engine: "tesseract".into(),
        provider: "local-tesseract".into(),
        model: Some("eng".into()),
        pixel_hash: format!("pixel-{timestamp}"),
        status: "succeeded".into(),
        error_code: None,
        content: Some(OcrContentInput {
            text: text.into(),
            blocks: vec![],
        }),
        cache_content_id: None,
        raw_regions: vec![],
    }
}
pub(super) fn history(
    connection: &Connection,
    chunk: &ProcessingChunkRow,
) -> crate::ai::OcrHistoryPage {
    get_ocr_history(
        connection,
        &GetOcrHistoryInput {
            recording_id: chunk.recording_id.clone(),
            source_track_id: Some("video-0".into()),
            limit: None,
            cursor: None,
        },
    )
    .unwrap()
}
