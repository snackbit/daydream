use rusqlite::{params, Connection, OptionalExtension};

#[derive(Clone, Debug, PartialEq)]
pub struct RecordingTrackRow {
    pub codec: Option<String>,
    pub created_at_ms: i64,
    pub id: String,
    pub kind: String,
    pub label: String,
    pub path: Option<String>,
    pub recording_id: String,
    pub source_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct UpsertRecordingTrackInput {
    pub codec: Option<String>,
    pub created_at_ms: i64,
    pub id: String,
    pub kind: String,
    pub label: String,
    pub path: Option<String>,
    pub recording_id: String,
    pub source_id: Option<String>,
}

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
pub struct CreateProcessingJobInput {
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
    pub track_id: Option<String>,
    pub updated_at_ms: i64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProcessingChunkRow {
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
    pub track_id: Option<String>,
    pub updated_at_ms: i64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CreateCaptureEventInput {
    pub id: String,
    pub kind: String,
    pub occurred_at_ms: i64,
    pub offset_ms: i64,
    pub payload_json: String,
    pub recording_id: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CaptureEventRow {
    pub id: String,
    pub kind: String,
    pub occurred_at_ms: i64,
    pub offset_ms: i64,
    pub payload_json: String,
    pub recording_id: String,
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
pub struct TranscriptWordRow {
    pub end_offset_ms: i64,
    pub id: String,
    pub probability: Option<f64>,
    pub start_offset_ms: i64,
    pub text: String,
    pub transcript_segment_id: String,
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
pub struct ActivityIntervalRow {
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

pub fn upsert_recording_track(
    connection: &Connection,
    input: &UpsertRecordingTrackInput,
) -> rusqlite::Result<RecordingTrackRow> {
    connection.execute(
        r#"
        INSERT INTO recording_tracks (
            id,
            recording_id,
            kind,
            label,
            source_id,
            path,
            codec,
            created_at_ms
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
        ON CONFLICT(id) DO UPDATE SET
            label = excluded.label,
            source_id = excluded.source_id,
            path = excluded.path,
            codec = excluded.codec
        "#,
        params![
            input.id,
            input.recording_id,
            input.kind,
            input.label,
            input.source_id,
            input.path,
            input.codec,
            input.created_at_ms,
        ],
    )?;

    get_recording_track(connection, &input.id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)
}

pub fn get_recording_track(
    connection: &Connection,
    id: &str,
) -> rusqlite::Result<Option<RecordingTrackRow>> {
    connection
        .query_row(
            r#"
            SELECT
                id,
                recording_id,
                kind,
                label,
                source_id,
                path,
                codec,
                created_at_ms
            FROM recording_tracks
            WHERE id = ?1
            "#,
            [id],
            recording_track_row,
        )
        .optional()
}

pub fn get_recording_track_by_kind(
    connection: &Connection,
    recording_id: &str,
    kind: &str,
) -> rusqlite::Result<Option<RecordingTrackRow>> {
    connection
        .query_row(
            r#"
            SELECT
                id,
                recording_id,
                kind,
                label,
                source_id,
                path,
                codec,
                created_at_ms
            FROM recording_tracks
            WHERE recording_id = ?1
                AND kind = ?2
            ORDER BY created_at_ms ASC, id ASC
            LIMIT 1
            "#,
            params![recording_id, kind],
            recording_track_row,
        )
        .optional()
}

pub fn list_recording_tracks(
    connection: &Connection,
    recording_id: &str,
) -> rusqlite::Result<Vec<RecordingTrackRow>> {
    let mut statement = connection.prepare(
        r#"
        SELECT
            id,
            recording_id,
            kind,
            label,
            source_id,
            path,
            codec,
            created_at_ms
        FROM recording_tracks
        WHERE recording_id = ?1
        ORDER BY kind ASC, created_at_ms ASC, id ASC
        "#,
    )?;
    let rows = statement.query_map([recording_id], recording_track_row)?;

    rows.collect()
}

pub fn recording_has_redactions(
    connection: &Connection,
    recording_id: &str,
) -> rusqlite::Result<bool> {
    connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM redactions WHERE recording_id = ?1)",
        [recording_id],
        |row| row.get::<_, bool>(0),
    )
}

pub fn create_processing_job(
    connection: &Connection,
    input: &CreateProcessingJobInput,
) -> rusqlite::Result<ProcessingJobRow> {
    connection.execute(
        r#"
        INSERT INTO processing_jobs (
            id,
            recording_id,
            kind,
            status,
            created_at_ms,
            updated_at_ms,
            error
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
        "#,
        params![
            input.id,
            input.recording_id,
            input.kind,
            input.status,
            input.created_at_ms,
            input.updated_at_ms,
            input.error,
        ],
    )?;

    get_processing_job(connection, &input.id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)
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

pub fn get_latest_processing_job_for_recording(
    connection: &Connection,
    recording_id: &str,
    kind: &str,
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
            WHERE recording_id = ?1
                AND kind = ?2
            ORDER BY updated_at_ms DESC, created_at_ms DESC
            LIMIT 1
            "#,
            params![recording_id, kind],
            processing_job_row,
        )
        .optional()
}

pub fn list_processing_jobs_by_status(
    connection: &Connection,
    kind: &str,
    statuses: &[&str],
    limit: Option<usize>,
) -> rusqlite::Result<Vec<ProcessingJobRow>> {
    if statuses.is_empty() || limit == Some(0) {
        return Ok(Vec::new());
    }
    let placeholders = (0..statuses.len())
        .map(|index| format!("?{}", index + 2))
        .collect::<Vec<_>>()
        .join(", ");
    let limit_clause = limit
        .map(|_| format!(" LIMIT ?{}", statuses.len() + 2))
        .unwrap_or_default();
    let sql = format!(
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
        WHERE kind = ?1
            AND status IN ({placeholders})
        ORDER BY updated_at_ms ASC, created_at_ms ASC
        {limit_clause}
        "#,
    );
    let limit_i64 = limit.map(|value| i64::try_from(value).unwrap_or(i64::MAX));
    let mut params: Vec<&dyn rusqlite::ToSql> = Vec::with_capacity(statuses.len() + 2);
    params.push(&kind);
    for status in statuses {
        params.push(status);
    }
    if let Some(value) = &limit_i64 {
        params.push(value);
    }
    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map(params.as_slice(), processing_job_row)?;

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

pub fn processing_job_chunk_progress(
    connection: &Connection,
    job_id: &str,
) -> rusqlite::Result<(i64, i64, f64)> {
    connection.query_row(
        r#"
        SELECT
            COUNT(*),
            COALESCE(SUM(CASE WHEN status = 'succeeded' THEN 1 ELSE 0 END), 0),
            COALESCE(AVG(progress), 0)
        FROM processing_chunks
        WHERE job_id = ?1
        "#,
        [job_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )
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
            track_id,
            created_at_ms,
            updated_at_ms
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
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
            input.track_id,
            input.created_at_ms,
            input.updated_at_ms,
        ],
    )?;

    get_processing_chunk(connection, &input.id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)
}

pub fn create_capture_event(
    connection: &Connection,
    input: &CreateCaptureEventInput,
) -> rusqlite::Result<()> {
    connection.execute(
        r#"
        INSERT INTO capture_events (
            id,
            recording_id,
            occurred_at_ms,
            offset_ms,
            kind,
            payload_json
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        "#,
        params![
            input.id,
            input.recording_id,
            input.occurred_at_ms,
            input.offset_ms,
            input.kind,
            input.payload_json,
        ],
    )?;

    Ok(())
}

pub fn list_capture_events(
    connection: &Connection,
    recording_id: &str,
    kinds: &[&str],
    start_offset_ms: i64,
    end_offset_ms: i64,
) -> rusqlite::Result<Vec<CaptureEventRow>> {
    if kinds.is_empty() {
        return Ok(Vec::new());
    }
    let placeholders = (0..kinds.len())
        .map(|index| format!("?{}", index + 4))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        r#"
        SELECT
            id,
            recording_id,
            occurred_at_ms,
            offset_ms,
            kind,
            payload_json
        FROM capture_events
        WHERE recording_id = ?1
            AND offset_ms >= ?2
            AND offset_ms <= ?3
            AND kind IN ({placeholders})
        ORDER BY offset_ms ASC, kind ASC, id ASC
        "#
    );
    let mut params: Vec<&dyn rusqlite::ToSql> = Vec::with_capacity(kinds.len() + 3);
    params.push(&recording_id);
    params.push(&start_offset_ms);
    params.push(&end_offset_ms);
    for kind in kinds {
        params.push(kind);
    }
    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map(params.as_slice(), capture_event_row)?;

    rows.collect()
}

pub fn get_processing_chunk(
    connection: &Connection,
    id: &str,
) -> rusqlite::Result<Option<ProcessingChunkRow>> {
    connection
        .query_row(
            r#"
            SELECT
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
                track_id,
                created_at_ms,
                updated_at_ms
            FROM processing_chunks
            WHERE id = ?1
            "#,
            [id],
            processing_chunk_row,
        )
        .optional()
}

pub fn list_processing_chunks(
    connection: &Connection,
    job_id: &str,
) -> rusqlite::Result<Vec<ProcessingChunkRow>> {
    let mut statement = connection.prepare(
        r#"
        SELECT
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
            track_id,
            created_at_ms,
            updated_at_ms
        FROM processing_chunks
        WHERE job_id = ?1
        ORDER BY start_offset_ms ASC, id ASC
        "#,
    )?;
    let rows = statement.query_map([job_id], processing_chunk_row)?;

    rows.collect()
}

pub fn list_processing_chunks_by_status(
    connection: &Connection,
    job_id: &str,
    statuses: &[&str],
    limit: Option<usize>,
) -> rusqlite::Result<Vec<ProcessingChunkRow>> {
    if statuses.is_empty() || limit == Some(0) {
        return Ok(Vec::new());
    }
    let placeholders = (0..statuses.len())
        .map(|index| format!("?{}", index + 2))
        .collect::<Vec<_>>()
        .join(", ");
    let limit_clause = limit
        .map(|_| format!(" LIMIT ?{}", statuses.len() + 2))
        .unwrap_or_default();
    let sql = format!(
        r#"
        SELECT
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
            track_id,
            created_at_ms,
            updated_at_ms
        FROM processing_chunks
        WHERE job_id = ?1
            AND status IN ({placeholders})
        ORDER BY start_offset_ms ASC, id ASC
        {limit_clause}
        "#,
    );
    let limit_i64 = limit.map(|value| i64::try_from(value).unwrap_or(i64::MAX));
    let mut params: Vec<&dyn rusqlite::ToSql> = Vec::with_capacity(statuses.len() + 2);
    params.push(&job_id);
    for status in statuses {
        params.push(status);
    }
    if let Some(value) = &limit_i64 {
        params.push(value);
    }
    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map(params.as_slice(), processing_chunk_row)?;

    rows.collect()
}

pub fn claim_next_queued_processing_chunk(
    connection: &Connection,
    job_id: &str,
    progress: f64,
    timestamp_ms: i64,
) -> rusqlite::Result<Option<ProcessingChunkRow>> {
    let chunk = list_processing_chunks_by_status(connection, job_id, &["queued"], Some(1))?
        .into_iter()
        .next();
    let Some(chunk) = chunk else {
        return Ok(None);
    };

    let updated = connection.execute(
        r#"
        UPDATE processing_chunks
        SET status = 'running',
            progress = ?2,
            error_code = NULL,
            attempt_count = attempt_count + 1,
            updated_at_ms = ?3
        WHERE id = ?1
            AND status = 'queued'
        "#,
        params![&chunk.id, progress, timestamp_ms],
    )?;
    if updated == 0 {
        return Ok(None);
    }

    get_processing_chunk(connection, &chunk.id)
}

pub fn update_processing_chunk_status(
    connection: &Connection,
    id: &str,
    status: &str,
    progress: f64,
    error_code: Option<&str>,
    timestamp_ms: i64,
) -> rusqlite::Result<ProcessingChunkRow> {
    connection.execute(
        r#"
        UPDATE processing_chunks
        SET status = ?2,
            progress = ?3,
            error_code = ?4,
            attempt_count = CASE WHEN ?2 = 'running' THEN attempt_count + 1 ELSE attempt_count END,
            updated_at_ms = ?5
        WHERE id = ?1
        "#,
        params![id, status, progress, error_code, timestamp_ms],
    )?;

    get_processing_chunk(connection, id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)
}

pub fn requeue_processing_chunks(
    connection: &Connection,
    job_id: &str,
    from_statuses: &[&str],
    timestamp_ms: i64,
) -> rusqlite::Result<usize> {
    if from_statuses.is_empty() {
        return Ok(0);
    }
    let placeholders = (0..from_statuses.len())
        .map(|index| format!("?{}", index + 3))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        r#"
        UPDATE processing_chunks
        SET status = 'queued',
            progress = 0,
            error_code = NULL,
            updated_at_ms = ?2
        WHERE job_id = ?1
            AND status IN ({placeholders})
        "#,
    );
    let mut params: Vec<&dyn rusqlite::ToSql> = Vec::with_capacity(from_statuses.len() + 2);
    params.push(&job_id);
    params.push(&timestamp_ms);
    for status in from_statuses {
        params.push(status);
    }

    connection.execute(&sql, params.as_slice())
}

pub fn cancel_processing_chunks(
    connection: &Connection,
    job_id: &str,
    from_statuses: &[&str],
    timestamp_ms: i64,
) -> rusqlite::Result<usize> {
    if from_statuses.is_empty() {
        return Ok(0);
    }
    let placeholders = (0..from_statuses.len())
        .map(|index| format!("?{}", index + 3))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        r#"
        UPDATE processing_chunks
        SET status = 'cancelled',
            error_code = 'ai_processing_cancelled',
            updated_at_ms = ?2
        WHERE job_id = ?1
            AND status IN ({placeholders})
        "#,
    );
    let mut params: Vec<&dyn rusqlite::ToSql> = Vec::with_capacity(from_statuses.len() + 2);
    params.push(&job_id);
    params.push(&timestamp_ms);
    for status in from_statuses {
        params.push(status);
    }

    connection.execute(&sql, params.as_slice())
}

pub fn delete_transcript_output_for_chunks(
    connection: &Connection,
    chunk_ids: &[String],
) -> rusqlite::Result<usize> {
    if chunk_ids.is_empty() {
        return Ok(0);
    }
    let placeholders = (0..chunk_ids.len())
        .map(|index| format!("?{}", index + 1))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!("DELETE FROM transcript_segments WHERE chunk_id IN ({placeholders})");
    let params = chunk_ids
        .iter()
        .map(|id| id as &dyn rusqlite::ToSql)
        .collect::<Vec<_>>();

    connection.execute(&sql, params.as_slice())
}

pub fn delete_activity_output_for_chunks(
    connection: &Connection,
    chunk_ids: &[String],
) -> rusqlite::Result<usize> {
    if chunk_ids.is_empty() {
        return Ok(0);
    }
    let placeholders = (0..chunk_ids.len())
        .map(|index| format!("?{}", index + 1))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!("DELETE FROM frame_activity_intervals WHERE chunk_id IN ({placeholders})");
    let params = chunk_ids
        .iter()
        .map(|id| id as &dyn rusqlite::ToSql)
        .collect::<Vec<_>>();

    connection.execute(&sql, params.as_slice())
}

pub fn retry_processing_job_chunks(
    connection: &Connection,
    job_id: &str,
    timestamp_ms: i64,
) -> rusqlite::Result<ProcessingJobRow> {
    let chunks =
        list_processing_chunks_by_status(connection, job_id, &["failed", "cancelled"], None)?;
    let chunk_ids = chunks
        .iter()
        .map(|chunk| chunk.id.clone())
        .collect::<Vec<_>>();
    delete_transcript_output_for_chunks(connection, &chunk_ids)?;
    delete_activity_output_for_chunks(connection, &chunk_ids)?;
    requeue_processing_chunks(connection, job_id, &["failed", "cancelled"], timestamp_ms)?;
    update_processing_job_status(connection, job_id, "queued", None, timestamp_ms)
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

    get_transcript_segment(connection, &input.id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)
}

pub fn get_transcript_segment(
    connection: &Connection,
    id: &str,
) -> rusqlite::Result<Option<TranscriptSegmentRow>> {
    connection
        .query_row(
            r#"
            SELECT
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
            FROM transcript_segments
            WHERE id = ?1
            "#,
            [id],
            transcript_segment_row,
        )
        .optional()
}

pub fn list_transcript_segments(
    connection: &Connection,
    recording_id: &str,
) -> rusqlite::Result<Vec<TranscriptSegmentRow>> {
    let mut statement = connection.prepare(
        r#"
        SELECT
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
        FROM transcript_segments
        WHERE recording_id = ?1
        ORDER BY track_id ASC, start_offset_ms ASC, end_offset_ms ASC, id ASC
        "#,
    )?;
    let rows = statement.query_map([recording_id], transcript_segment_row)?;

    rows.collect()
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

pub fn list_transcript_words(
    connection: &Connection,
    transcript_segment_id: &str,
) -> rusqlite::Result<Vec<TranscriptWordRow>> {
    let mut statement = connection.prepare(
        r#"
        SELECT
            id,
            transcript_segment_id,
            start_offset_ms,
            end_offset_ms,
            text,
            probability
        FROM transcript_words
        WHERE transcript_segment_id = ?1
        ORDER BY start_offset_ms ASC, end_offset_ms ASC, id ASC
        "#,
    )?;
    let rows = statement.query_map([transcript_segment_id], transcript_word_row)?;

    rows.collect()
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

pub fn list_activity_intervals(
    connection: &Connection,
    recording_id: &str,
) -> rusqlite::Result<Vec<ActivityIntervalRow>> {
    let mut statement = connection.prepare(
        r#"
        SELECT
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
        FROM frame_activity_intervals
        WHERE recording_id = ?1
        ORDER BY source_kind ASC, start_offset_ms ASC, end_offset_ms ASC, id ASC
        "#,
    )?;
    let rows = statement.query_map([recording_id], activity_interval_row)?;

    rows.collect()
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

fn recording_track_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RecordingTrackRow> {
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

fn processing_chunk_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProcessingChunkRow> {
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
        track_id: row.get(11)?,
        created_at_ms: row.get(12)?,
        updated_at_ms: row.get(13)?,
    })
}

fn capture_event_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<CaptureEventRow> {
    Ok(CaptureEventRow {
        id: row.get(0)?,
        recording_id: row.get(1)?,
        occurred_at_ms: row.get(2)?,
        offset_ms: row.get(3)?,
        kind: row.get(4)?,
        payload_json: row.get(5)?,
    })
}

fn transcript_segment_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<TranscriptSegmentRow> {
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

fn activity_interval_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ActivityIntervalRow> {
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

fn transcript_word_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<TranscriptWordRow> {
    Ok(TranscriptWordRow {
        id: row.get(0)?,
        transcript_segment_id: row.get(1)?,
        start_offset_ms: row.get(2)?,
        end_offset_ms: row.get(3)?,
        text: row.get(4)?,
        probability: row.get(5)?,
    })
}
