use super::types::*;

pub(super) fn ai_model_asset_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<AiModelAssetRow> {
    Ok(AiModelAssetRow {
        id: row.get(0)?,
        provider: row.get(1)?,
        display_label: row.get(2)?,
        local_path: row.get(3)?,
        size_bytes: row.get(4)?,
        checksum_algorithm: row.get(5)?,
        checksum: row.get(6)?,
        etag: row.get(7)?,
        source_url: row.get(8)?,
        downloaded_bytes: row.get(9)?,
        total_bytes: row.get(10)?,
        error_code: row.get(11)?,
        verification_status: row.get(12)?,
        download_status: row.get(13)?,
        self_test_status: row.get(14)?,
        created_at_ms: row.get(15)?,
        updated_at_ms: row.get(16)?,
    })
}

pub(super) fn ocr_region_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<OcrRegionRow> {
    Ok(OcrRegionRow {
        id: row.get(0)?,
        recording_id: row.get(1)?,
        frame_timestamp_ms: row.get(2)?,
        source_frame_ref: row.get(3)?,
        region_x: row.get(4)?,
        region_y: row.get(5)?,
        region_width: row.get(6)?,
        region_height: row.get(7)?,
        text: row.get(8)?,
        confidence: row.get(9)?,
        provider: row.get(10)?,
        model: row.get(11)?,
        job_id: row.get(12)?,
        chunk_id: row.get(13)?,
    })
}

pub(super) fn vlm_observation_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<VlmObservationRow> {
    Ok(VlmObservationRow {
        id: row.get(0)?,
        recording_id: row.get(1)?,
        frame_timestamp_ms: row.get(2)?,
        source_frame_ref: row.get(3)?,
        region_x: row.get(4)?,
        region_y: row.get(5)?,
        region_width: row.get(6)?,
        region_height: row.get(7)?,
        text: row.get(8)?,
        confidence: row.get(9)?,
        provider: row.get(10)?,
        model: row.get(11)?,
        prompt_version: row.get(12)?,
        observation_kind: row.get(13)?,
        job_id: row.get(14)?,
        chunk_id: row.get(15)?,
    })
}

pub(super) fn visual_frame_processing_state_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<VisualFrameProcessingStateRow> {
    Ok(VisualFrameProcessingStateRow {
        id: row.get(0)?,
        recording_id: row.get(1)?,
        job_id: row.get(2)?,
        chunk_id: row.get(3)?,
        source_kind: row.get(4)?,
        segment_id: row.get(5)?,
        frame_timestamp_ms: row.get(6)?,
        source_frame_ref: row.get(7)?,
        region_x: row.get(8)?,
        region_y: row.get(9)?,
        region_width: row.get(10)?,
        region_height: row.get(11)?,
        status: row.get(12)?,
        progress: row.get(13)?,
        error_code: row.get(14)?,
        output_count: row.get(15)?,
        created_at_ms: row.get(16)?,
        updated_at_ms: row.get(17)?,
    })
}

pub(super) fn recording_track_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RecordingTrackRow> {
    Ok(RecordingTrackRow {
        id: row.get(0)?,
        recording_id: row.get(1)?,
        kind: row.get(2)?,
        label: row.get(3)?,
        source_id: row.get(4)?,
        path: row.get(5)?,
        codec: row.get(6)?,
        created_at_ms: row.get(7)?,
    })
}

pub(super) fn processing_job_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProcessingJobRow> {
    Ok(ProcessingJobRow {
        id: row.get(0)?,
        recording_id: row.get(1)?,
        kind: row.get(2)?,
        status: row.get(3)?,
        created_at_ms: row.get(4)?,
        updated_at_ms: row.get(5)?,
        error: row.get(6)?,
    })
}

pub(super) fn processing_chunk_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<ProcessingChunkRow> {
    Ok(ProcessingChunkRow {
        id: row.get(0)?,
        job_id: row.get(1)?,
        recording_id: row.get(2)?,
        source_kind: row.get(3)?,
        segment_id: row.get(4)?,
        start_offset_ms: row.get(5)?,
        end_offset_ms: row.get(6)?,
        status: row.get(7)?,
        attempt_count: row.get(8)?,
        progress: row.get(9)?,
        error_code: row.get(10)?,
        result_code: row.get(11)?,
        track_id: row.get(12)?,
        created_at_ms: row.get(13)?,
        updated_at_ms: row.get(14)?,
    })
}

pub(super) fn capture_event_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<CaptureEventRow> {
    Ok(CaptureEventRow {
        id: row.get(0)?,
        recording_id: row.get(1)?,
        occurred_at_ms: row.get(2)?,
        offset_ms: row.get(3)?,
        kind: row.get(4)?,
        payload_json: row.get(5)?,
    })
}

pub(super) fn transcript_segment_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<TranscriptSegmentRow> {
    Ok(TranscriptSegmentRow {
        id: row.get(0)?,
        recording_id: row.get(1)?,
        segment_id: row.get(2)?,
        track_id: row.get(3)?,
        source_role: row.get(4)?,
        start_offset_ms: row.get(5)?,
        end_offset_ms: row.get(6)?,
        provider: row.get(7)?,
        model: row.get(8)?,
        text: row.get(9)?,
        confidence: row.get(10)?,
        job_id: row.get(11)?,
        chunk_id: row.get(12)?,
        created_at_ms: row.get(13)?,
    })
}

pub(super) fn activity_interval_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<ActivityIntervalRow> {
    Ok(ActivityIntervalRow {
        id: row.get(0)?,
        recording_id: row.get(1)?,
        source_kind: row.get(2)?,
        state: row.get(3)?,
        start_offset_ms: row.get(4)?,
        end_offset_ms: row.get(5)?,
        score: row.get(6)?,
        regions_json: row.get(7)?,
        job_id: row.get(8)?,
        chunk_id: row.get(9)?,
    })
}

pub(super) fn transcript_word_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<TranscriptWordRow> {
    Ok(TranscriptWordRow {
        id: row.get(0)?,
        transcript_segment_id: row.get(1)?,
        start_offset_ms: row.get(2)?,
        end_offset_ms: row.get(3)?,
        text: row.get(4)?,
        probability: row.get(5)?,
    })
}
