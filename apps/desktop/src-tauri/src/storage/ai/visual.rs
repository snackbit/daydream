use rusqlite::{params, Connection, OptionalExtension};

use super::{
    rows::{ocr_region_row, visual_frame_processing_state_row, vlm_observation_row},
    types::*,
};

pub fn create_ocr_region(
    connection: &Connection,
    input: &CreateVisualRegionInput,
) -> rusqlite::Result<()> {
    connection.execute(
        r#"
        INSERT INTO ocr_regions (
            id,
            recording_id,
            frame_timestamp_ms,
            source_frame_ref,
            region_x,
            region_y,
            region_width,
            region_height,
            text,
            confidence,
            provider,
            model,
            job_id,
            chunk_id
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
        "#,
        params![
            input.id,
            input.recording_id,
            input.frame_timestamp_ms,
            input.source_frame_ref,
            input.region_x,
            input.region_y,
            input.region_width,
            input.region_height,
            input.text,
            input.confidence,
            input.provider,
            input.model,
            input.job_id,
            input.chunk_id,
        ],
    )?;

    Ok(())
}

pub fn create_vlm_observation(
    connection: &Connection,
    input: &CreateVisualRegionInput,
) -> rusqlite::Result<()> {
    connection.execute(
        r#"
        INSERT INTO vlm_observations (
            id,
            recording_id,
            frame_timestamp_ms,
            source_frame_ref,
            region_x,
            region_y,
            region_width,
            region_height,
            text,
            confidence,
            provider,
            model,
            prompt_version,
            observation_kind,
            job_id,
            chunk_id
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)
        "#,
        params![
            input.id,
            input.recording_id,
            input.frame_timestamp_ms,
            input.source_frame_ref,
            input.region_x,
            input.region_y,
            input.region_width,
            input.region_height,
            input.text,
            input.confidence,
            input.provider,
            input.model,
            input
                .prompt_version
                .as_deref()
                .unwrap_or("m2-visual-observation-v1"),
            input
                .observation_kind
                .as_deref()
                .unwrap_or("screen-observation"),
            input.job_id,
            input.chunk_id,
        ],
    )?;

    Ok(())
}

pub fn create_frame_similarity_entry(
    connection: &Connection,
    input: &CreateFrameSimilarityEntryInput,
) -> rusqlite::Result<()> {
    connection.execute(
        r#"
        INSERT INTO frame_similarity_entries (
            id,
            recording_id,
            source_frame_ref,
            region_x,
            region_y,
            region_width,
            region_height,
            signature,
            last_processed_timestamp_ms,
            job_id,
            chunk_id
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
        "#,
        params![
            input.id,
            input.recording_id,
            input.source_frame_ref,
            input.region_x,
            input.region_y,
            input.region_width,
            input.region_height,
            input.signature,
            input.last_processed_timestamp_ms,
            input.job_id,
            input.chunk_id,
        ],
    )?;

    Ok(())
}

pub fn list_ocr_regions(
    connection: &Connection,
    recording_id: &str,
) -> rusqlite::Result<Vec<OcrRegionRow>> {
    let mut statement = connection.prepare(
        r#"
        SELECT
            id,
            recording_id,
            frame_timestamp_ms,
            source_frame_ref,
            region_x,
            region_y,
            region_width,
            region_height,
            text,
            confidence,
            provider,
            model,
            job_id,
            chunk_id
        FROM ocr_regions
        WHERE recording_id = ?1
        ORDER BY frame_timestamp_ms ASC, region_y ASC, region_x ASC, id ASC
        "#,
    )?;
    let rows = statement.query_map([recording_id], ocr_region_row)?;

    rows.collect()
}

pub fn list_vlm_observations(
    connection: &Connection,
    recording_id: &str,
) -> rusqlite::Result<Vec<VlmObservationRow>> {
    let mut statement = connection.prepare(
        r#"
        SELECT
            id,
            recording_id,
            frame_timestamp_ms,
            source_frame_ref,
            region_x,
            region_y,
            region_width,
            region_height,
            text,
            confidence,
            provider,
            model,
            prompt_version,
            observation_kind,
            job_id,
            chunk_id
        FROM vlm_observations
        WHERE recording_id = ?1
        AND (NOT EXISTS(SELECT 1 FROM processing_replacement_chunks rc JOIN processing_replacement_groups rg ON rg.id=rc.group_id WHERE rc.chunk_id=vlm_observations.chunk_id AND rg.committed=0) OR EXISTS(SELECT 1 FROM processing_replacement_frames rf WHERE rf.chunk_id=vlm_observations.chunk_id AND rf.frame_ms=vlm_observations.frame_timestamp_ms))
        ORDER BY frame_timestamp_ms ASC, observation_kind ASC, id ASC
        "#,
    )?;
    let rows = statement.query_map([recording_id], vlm_observation_row)?;

    rows.collect()
}

pub fn upsert_visual_frame_processing_state(
    connection: &Connection,
    input: &CreateVisualFrameProcessingStateInput,
) -> rusqlite::Result<VisualFrameProcessingStateRow> {
    connection.execute(
        r#"
        INSERT INTO visual_frame_processing_states (
            id,
            recording_id,
            job_id,
            chunk_id,
            source_kind,
            segment_id,
            frame_timestamp_ms,
            source_frame_ref,
            region_x,
            region_y,
            region_width,
            region_height,
            status,
            progress,
            error_code,
            output_count,
            created_at_ms,
            updated_at_ms
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)
        ON CONFLICT(chunk_id, source_kind, source_frame_ref) DO UPDATE SET
            segment_id = excluded.segment_id,
            frame_timestamp_ms = excluded.frame_timestamp_ms,
            region_x = excluded.region_x,
            region_y = excluded.region_y,
            region_width = excluded.region_width,
            region_height = excluded.region_height,
            status = excluded.status,
            progress = excluded.progress,
            error_code = excluded.error_code,
            output_count = excluded.output_count,
            updated_at_ms = excluded.updated_at_ms
        "#,
        params![
            input.id,
            input.recording_id,
            input.job_id,
            input.chunk_id,
            input.source_kind,
            input.segment_id,
            input.frame_timestamp_ms,
            input.source_frame_ref,
            input.region_x,
            input.region_y,
            input.region_width,
            input.region_height,
            input.status,
            input.progress,
            input.error_code,
            input.output_count,
            input.created_at_ms,
            input.updated_at_ms,
        ],
    )?;

    get_visual_frame_processing_state(connection, &input.id)?
        .or_else(|| {
            get_visual_frame_processing_state_by_ref(
                connection,
                &input.chunk_id,
                &input.source_kind,
                &input.source_frame_ref,
            )
            .ok()
            .flatten()
        })
        .ok_or(rusqlite::Error::QueryReturnedNoRows)
}

pub fn get_visual_frame_processing_state(
    connection: &Connection,
    id: &str,
) -> rusqlite::Result<Option<VisualFrameProcessingStateRow>> {
    let mut statement = connection.prepare(VISUAL_FRAME_PROCESSING_STATE_SELECT)?;
    statement
        .query_row([id], visual_frame_processing_state_row)
        .optional()
}

pub fn get_visual_frame_processing_state_by_ref(
    connection: &Connection,
    chunk_id: &str,
    source_kind: &str,
    source_frame_ref: &str,
) -> rusqlite::Result<Option<VisualFrameProcessingStateRow>> {
    let mut statement = connection.prepare(
        r#"
        SELECT
            id,
            recording_id,
            job_id,
            chunk_id,
            source_kind,
            segment_id,
            frame_timestamp_ms,
            source_frame_ref,
            region_x,
            region_y,
            region_width,
            region_height,
            status,
            progress,
            error_code,
            output_count,
            created_at_ms,
            updated_at_ms
        FROM visual_frame_processing_states
        WHERE chunk_id = ?1
            AND source_kind = ?2
            AND source_frame_ref = ?3
        "#,
    )?;
    statement
        .query_row(
            params![chunk_id, source_kind, source_frame_ref],
            visual_frame_processing_state_row,
        )
        .optional()
}

pub fn list_visual_frame_processing_states(
    connection: &Connection,
    recording_id: &str,
) -> rusqlite::Result<Vec<VisualFrameProcessingStateRow>> {
    let mut statement = connection.prepare(
        r#"
        SELECT
            id,
            recording_id,
            job_id,
            chunk_id,
            source_kind,
            segment_id,
            frame_timestamp_ms,
            source_frame_ref,
            region_x,
            region_y,
            region_width,
            region_height,
            status,
            progress,
            error_code,
            output_count,
            created_at_ms,
            updated_at_ms
        FROM visual_frame_processing_states
        WHERE recording_id = ?1
        ORDER BY frame_timestamp_ms ASC, source_kind ASC, source_frame_ref ASC, id ASC
        "#,
    )?;
    let rows = statement.query_map([recording_id], visual_frame_processing_state_row)?;

    rows.collect()
}

pub fn update_visual_frame_processing_state_status(
    connection: &Connection,
    id: &str,
    status: &str,
    progress: f64,
    error_code: Option<&str>,
    output_count: i64,
    timestamp_ms: i64,
) -> rusqlite::Result<VisualFrameProcessingStateRow> {
    connection.execute(
        r#"
        UPDATE visual_frame_processing_states
        SET status = ?2,
            progress = ?3,
            error_code = ?4,
            output_count = ?5,
            updated_at_ms = ?6
        WHERE id = ?1
            AND status IN ('queued', 'running')
        "#,
        params![id, status, progress, error_code, output_count, timestamp_ms],
    )?;

    get_visual_frame_processing_state(connection, id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)
}

pub fn cancel_visual_frame_processing_states_for_chunk(
    connection: &Connection,
    chunk_id: &str,
    timestamp_ms: i64,
) -> rusqlite::Result<usize> {
    connection.execute(
        r#"
        UPDATE visual_frame_processing_states
        SET status = 'cancelled',
            error_code = 'ai_processing_cancelled',
            updated_at_ms = ?2
        WHERE chunk_id = ?1
            AND status IN ('queued', 'running')
        "#,
        params![chunk_id, timestamp_ms],
    )
}

const VISUAL_FRAME_PROCESSING_STATE_SELECT: &str = r#"
    SELECT
        id,
        recording_id,
        job_id,
        chunk_id,
        source_kind,
        segment_id,
        frame_timestamp_ms,
        source_frame_ref,
        region_x,
        region_y,
        region_width,
        region_height,
        status,
        progress,
        error_code,
        output_count,
        created_at_ms,
        updated_at_ms
    FROM visual_frame_processing_states
    WHERE id = ?1
    "#;
