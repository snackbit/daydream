use rusqlite::{params, Connection};

use super::{
    rows::{ocr_region_row, vlm_observation_row},
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
        ORDER BY frame_timestamp_ms ASC, observation_kind ASC, id ASC
        "#,
    )?;
    let rows = statement.query_map([recording_id], vlm_observation_row)?;

    rows.collect()
}
