use rusqlite::{params, Connection, OptionalExtension};

#[derive(Clone, Debug, PartialEq)]
pub struct SaveAiModelAssetInput {
    pub checksum: Option<String>,
    pub checksum_algorithm: Option<String>,
    pub display_label: String,
    pub downloaded_bytes: i64,
    pub download_status: String,
    pub etag: Option<String>,
    pub error_code: Option<String>,
    pub id: String,
    pub local_path: Option<String>,
    pub provider: String,
    pub self_test_status: String,
    pub size_bytes: Option<i64>,
    pub source_url: Option<String>,
    pub timestamp_ms: i64,
    pub total_bytes: Option<i64>,
    pub verification_status: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AiModelAssetRow {
    pub checksum: Option<String>,
    pub checksum_algorithm: Option<String>,
    pub created_at_ms: i64,
    pub display_label: String,
    pub downloaded_bytes: i64,
    pub download_status: String,
    pub etag: Option<String>,
    pub error_code: Option<String>,
    pub id: String,
    pub local_path: Option<String>,
    pub provider: String,
    pub self_test_status: String,
    pub size_bytes: Option<i64>,
    pub source_url: Option<String>,
    pub total_bytes: Option<i64>,
    pub updated_at_ms: i64,
    pub verification_status: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProcessingJobRow {
    pub created_at_ms: i64,
    pub error: Option<String>,
    pub id: String,
    pub kind: String,
    pub recording_id: String,
    pub status: String,
    pub updated_at_ms: i64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CreateProcessingChunkInput {
    pub attempt_count: i64,
    pub created_at_ms: i64,
    pub end_offset_ms: Option<i64>,
    pub error_code: Option<String>,
    pub id: String,
    pub job_id: String,
    pub progress: f64,
    pub recording_id: String,
    pub segment_id: Option<String>,
    pub source_kind: String,
    pub start_offset_ms: Option<i64>,
    pub status: String,
    pub updated_at_ms: i64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProcessingChunkRow {
    pub id: String,
    pub job_id: String,
    pub recording_id: String,
    pub source_kind: String,
    pub status: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CreateTranscriptSegmentInput {
    pub chunk_id: String,
    pub confidence: Option<f64>,
    pub created_at_ms: i64,
    pub end_offset_ms: i64,
    pub id: String,
    pub job_id: String,
    pub model: Option<String>,
    pub provider: String,
    pub recording_id: String,
    pub segment_id: String,
    pub source_role: String,
    pub start_offset_ms: i64,
    pub text: String,
    pub track_id: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TranscriptSegmentRow {
    pub chunk_id: String,
    pub id: String,
    pub job_id: String,
    pub recording_id: String,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CreateTranscriptWordInput {
    pub end_offset_ms: i64,
    pub id: String,
    pub probability: Option<f64>,
    pub start_offset_ms: i64,
    pub text: String,
    pub transcript_segment_id: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CreateActivityIntervalInput {
    pub chunk_id: Option<String>,
    pub end_offset_ms: i64,
    pub id: String,
    pub job_id: String,
    pub recording_id: String,
    pub regions_json: Option<String>,
    pub score: Option<f64>,
    pub source_kind: String,
    pub start_offset_ms: i64,
    pub state: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CreateVisualRegionInput {
    pub chunk_id: String,
    pub confidence: Option<f64>,
    pub frame_timestamp_ms: i64,
    pub id: String,
    pub job_id: String,
    pub model: Option<String>,
    pub prompt_version: Option<String>,
    pub provider: String,
    pub recording_id: String,
    pub region_height: i64,
    pub region_width: i64,
    pub region_x: i64,
    pub region_y: i64,
    pub source_frame_ref: String,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CreateFrameSimilarityEntryInput {
    pub chunk_id: Option<String>,
    pub id: String,
    pub job_id: String,
    pub last_processed_timestamp_ms: i64,
    pub recording_id: String,
    pub region_height: i64,
    pub region_width: i64,
    pub region_x: i64,
    pub region_y: i64,
    pub signature: String,
    pub source_frame_ref: String,
}

pub fn save_ai_model_asset(
    connection: &Connection,
    input: &SaveAiModelAssetInput,
) -> rusqlite::Result<AiModelAssetRow> {
    connection.execute(
        r#"
        INSERT INTO ai_model_assets (
            id,
            provider,
            display_label,
            local_path,
            size_bytes,
            checksum_algorithm,
            checksum,
            etag,
            source_url,
            downloaded_bytes,
            total_bytes,
            error_code,
            verification_status,
            download_status,
            self_test_status,
            created_at_ms,
            updated_at_ms
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?16)
        ON CONFLICT(id) DO UPDATE SET
            provider = excluded.provider,
            display_label = excluded.display_label,
            local_path = excluded.local_path,
            size_bytes = excluded.size_bytes,
            checksum_algorithm = excluded.checksum_algorithm,
            checksum = excluded.checksum,
            etag = excluded.etag,
            source_url = excluded.source_url,
            downloaded_bytes = excluded.downloaded_bytes,
            total_bytes = excluded.total_bytes,
            error_code = excluded.error_code,
            verification_status = excluded.verification_status,
            download_status = excluded.download_status,
            self_test_status = excluded.self_test_status,
            updated_at_ms = excluded.updated_at_ms
        "#,
        params![
            input.id,
            input.provider,
            input.display_label,
            input.local_path,
            input.size_bytes,
            input.checksum_algorithm,
            input.checksum,
            input.etag,
            input.source_url,
            input.downloaded_bytes,
            input.total_bytes,
            input.error_code,
            input.verification_status,
            input.download_status,
            input.self_test_status,
            input.timestamp_ms,
        ],
    )?;

    get_ai_model_asset(connection, &input.id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)
}

pub fn get_ai_model_asset(
    connection: &Connection,
    id: &str,
) -> rusqlite::Result<Option<AiModelAssetRow>> {
    connection
        .query_row(
            r#"
            SELECT
                id,
                provider,
                display_label,
                local_path,
                size_bytes,
                checksum_algorithm,
                checksum,
                etag,
                source_url,
                downloaded_bytes,
                total_bytes,
                error_code,
                verification_status,
                download_status,
                self_test_status,
                created_at_ms,
                updated_at_ms
            FROM ai_model_assets
            WHERE id = ?1
            "#,
            [id],
            ai_model_asset_row,
        )
        .optional()
}

pub fn list_ai_model_assets(connection: &Connection) -> rusqlite::Result<Vec<AiModelAssetRow>> {
    let mut statement = connection.prepare(
        r#"
        SELECT
            id,
            provider,
            display_label,
            local_path,
            size_bytes,
            checksum_algorithm,
            checksum,
            etag,
            source_url,
            downloaded_bytes,
            total_bytes,
            error_code,
            verification_status,
            download_status,
            self_test_status,
            created_at_ms,
            updated_at_ms
        FROM ai_model_assets
        ORDER BY id
        "#,
    )?;

    let rows = statement.query_map([], ai_model_asset_row)?;

    rows.collect()
}

pub fn delete_ai_model_asset(connection: &Connection, id: &str) -> rusqlite::Result<()> {
    connection.execute("DELETE FROM ai_model_assets WHERE id = ?1", [id])?;

    Ok(())
}

pub fn list_processing_jobs(
    connection: &Connection,
    recording_id: Option<&str>,
) -> rusqlite::Result<Vec<ProcessingJobRow>> {
    let sql = r#"
        SELECT
            id,
            recording_id,
            kind,
            status,
            created_at_ms,
            updated_at_ms,
            error
        FROM processing_jobs
        WHERE (?1 IS NULL OR recording_id = ?1)
        ORDER BY updated_at_ms DESC, created_at_ms DESC
        "#;
    let mut statement = connection.prepare(sql)?;
    let rows = statement.query_map([recording_id], processing_job_row)?;

    rows.collect()
}

pub fn get_processing_job(
    connection: &Connection,
    id: &str,
) -> rusqlite::Result<Option<ProcessingJobRow>> {
    connection
        .query_row(
            r#"
            SELECT
                id,
                recording_id,
                kind,
                status,
                created_at_ms,
                updated_at_ms,
                error
            FROM processing_jobs
            WHERE id = ?1
            "#,
            [id],
            processing_job_row,
        )
        .optional()
}

pub fn update_processing_job_status(
    connection: &Connection,
    id: &str,
    status: &str,
    error: Option<&str>,
    timestamp_ms: i64,
) -> rusqlite::Result<ProcessingJobRow> {
    connection.execute(
        r#"
        UPDATE processing_jobs
        SET status = ?2,
            error = ?3,
            updated_at_ms = ?4
        WHERE id = ?1
        "#,
        params![id, status, error, timestamp_ms],
    )?;

    get_processing_job(connection, id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)
}

pub fn create_processing_chunk(
    connection: &Connection,
    input: &CreateProcessingChunkInput,
) -> rusqlite::Result<ProcessingChunkRow> {
    connection.execute(
        r#"
        INSERT INTO processing_chunks (
            id,
            job_id,
            recording_id,
            source_kind,
            segment_id,
            start_offset_ms,
            end_offset_ms,
            status,
            attempt_count,
            progress,
            error_code,
            created_at_ms,
            updated_at_ms
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
        "#,
        params![
            input.id,
            input.job_id,
            input.recording_id,
            input.source_kind,
            input.segment_id,
            input.start_offset_ms,
            input.end_offset_ms,
            input.status,
            input.attempt_count,
            input.progress,
            input.error_code,
            input.created_at_ms,
            input.updated_at_ms,
        ],
    )?;

    connection.query_row(
        "SELECT id, job_id, recording_id, source_kind, status FROM processing_chunks WHERE id = ?1",
        [&input.id],
        |row| {
            Ok(ProcessingChunkRow {
                id: row.get(0)?,
                job_id: row.get(1)?,
                recording_id: row.get(2)?,
                source_kind: row.get(3)?,
                status: row.get(4)?,
            })
        },
    )
}

pub fn create_transcript_segment(
    connection: &Connection,
    input: &CreateTranscriptSegmentInput,
) -> rusqlite::Result<TranscriptSegmentRow> {
    connection.execute(
        r#"
        INSERT INTO transcript_segments (
            id,
            recording_id,
            segment_id,
            track_id,
            source_role,
            start_offset_ms,
            end_offset_ms,
            provider,
            model,
            text,
            confidence,
            job_id,
            chunk_id,
            created_at_ms
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
        "#,
        params![
            input.id,
            input.recording_id,
            input.segment_id,
            input.track_id,
            input.source_role,
            input.start_offset_ms,
            input.end_offset_ms,
            input.provider,
            input.model,
            input.text,
            input.confidence,
            input.job_id,
            input.chunk_id,
            input.created_at_ms,
        ],
    )?;

    connection.query_row(
        "SELECT id, recording_id, text, job_id, chunk_id FROM transcript_segments WHERE id = ?1",
        [&input.id],
        |row| {
            Ok(TranscriptSegmentRow {
                id: row.get(0)?,
                recording_id: row.get(1)?,
                text: row.get(2)?,
                job_id: row.get(3)?,
                chunk_id: row.get(4)?,
            })
        },
    )
}

pub fn create_transcript_word(
    connection: &Connection,
    input: &CreateTranscriptWordInput,
) -> rusqlite::Result<()> {
    connection.execute(
        r#"
        INSERT INTO transcript_words (
            id,
            transcript_segment_id,
            start_offset_ms,
            end_offset_ms,
            text,
            probability
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        "#,
        params![
            input.id,
            input.transcript_segment_id,
            input.start_offset_ms,
            input.end_offset_ms,
            input.text,
            input.probability,
        ],
    )?;

    Ok(())
}

pub fn create_activity_interval(
    connection: &Connection,
    input: &CreateActivityIntervalInput,
) -> rusqlite::Result<()> {
    connection.execute(
        r#"
        INSERT INTO frame_activity_intervals (
            id,
            recording_id,
            source_kind,
            state,
            start_offset_ms,
            end_offset_ms,
            score,
            regions_json,
            job_id,
            chunk_id
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
        "#,
        params![
            input.id,
            input.recording_id,
            input.source_kind,
            input.state,
            input.start_offset_ms,
            input.end_offset_ms,
            input.score,
            input.regions_json,
            input.job_id,
            input.chunk_id,
        ],
    )?;

    Ok(())
}

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
            job_id,
            chunk_id
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
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

fn ai_model_asset_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<AiModelAssetRow> {
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

fn processing_job_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProcessingJobRow> {
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
