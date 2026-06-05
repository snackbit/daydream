use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
};

use rusqlite::{params, Connection, OptionalExtension};
use rusqlite_migration::{Migrations, M};
use serde::Serialize;
use tauri::Manager;

const DATABASE_FILE_NAME: &str = "daydream.sqlite3";
const CORE_TABLES: [&str; 9] = [
    "recordings",
    "recording_tracks",
    "recording_segments",
    "recorder_scenes",
    "recorder_scene_sources",
    "recorder_portal_restore_tokens",
    "redactions",
    "capture_events",
    "processing_jobs",
];

pub struct AppStorage {
    connection: Mutex<Connection>,
    path: PathBuf,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseDiagnostics {
    available: bool,
    error: Option<String>,
    path: String,
    schema_version: Option<i64>,
    table_counts: BTreeMap<String, i64>,
}

pub fn initialize(app: &tauri::AppHandle) -> Result<AppStorage, Box<dyn std::error::Error>> {
    let app_data_dir = app.path().app_data_dir()?;
    fs::create_dir_all(&app_data_dir)?;

    let db_path = app_data_dir.join(DATABASE_FILE_NAME);
    let mut connection = open_connection(&db_path)?;
    apply_migrations(&mut connection)?;

    Ok(AppStorage {
        connection: Mutex::new(connection),
        path: db_path,
    })
}

pub fn collect_database_diagnostics(storage: &AppStorage) -> DatabaseDiagnostics {
    match storage.connection.lock() {
        Ok(connection) => match collect_database_diagnostics_from_connection(&connection) {
            Ok((schema_version, table_counts)) => DatabaseDiagnostics {
                available: true,
                error: None,
                path: path_to_string(&storage.path),
                schema_version: Some(schema_version),
                table_counts,
            },
            Err(error) => DatabaseDiagnostics {
                available: false,
                error: Some(error.to_string()),
                path: path_to_string(&storage.path),
                schema_version: None,
                table_counts: BTreeMap::new(),
            },
        },
        Err(error) => DatabaseDiagnostics {
            available: false,
            error: Some(format!("Database connection lock is poisoned: {error}")),
            path: path_to_string(&storage.path),
            schema_version: None,
            table_counts: BTreeMap::new(),
        },
    }
}

impl AppStorage {
    pub fn with_connection<T>(
        &self,
        callback: impl FnOnce(&Connection) -> rusqlite::Result<T>,
    ) -> rusqlite::Result<T> {
        let connection = self
            .connection
            .lock()
            .map_err(|_error| rusqlite::Error::InvalidQuery)?;

        callback(&connection)
    }

    pub fn with_connection_mut<T>(
        &self,
        callback: impl FnOnce(&mut Connection) -> rusqlite::Result<T>,
    ) -> rusqlite::Result<T> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_error| rusqlite::Error::InvalidQuery)?;

        callback(&mut connection)
    }
}

#[cfg(test)]
pub fn test_storage(path: PathBuf) -> AppStorage {
    let mut connection = Connection::open_in_memory().expect("open in-memory sqlite database");
    enable_connection_pragmas(&connection).expect("enable sqlite pragmas");
    apply_migrations(&mut connection).expect("apply migrations");

    AppStorage {
        connection: Mutex::new(connection),
        path,
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CreateRecordingInput {
    pub id: String,
    pub active_scene_id: Option<String>,
    pub created_at_ms: i64,
    pub default_fps: f64,
    pub notes: Option<String>,
    pub output_root: Option<String>,
    pub quality_label: String,
    pub storage_dir: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RecordingRow {
    pub id: String,
    pub active_scene_id: Option<String>,
    pub ended_at_ms: Option<i64>,
    pub output_root: Option<String>,
    pub quality_label: String,
    pub started_at_ms: Option<i64>,
    pub status: String,
    pub storage_dir: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SaveRecorderSceneInput {
    pub canvas_height: i64,
    pub canvas_width: i64,
    pub desktop_audio_source_id: Option<String>,
    pub id: String,
    pub layout_mode: String,
    pub microphone_audio_source_id: Option<String>,
    pub name: String,
    pub sources: Vec<SaveRecorderSceneSourceInput>,
    pub timestamp_ms: i64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SaveRecorderSceneSourceInput {
    pub crop_bottom: i64,
    pub crop_left: i64,
    pub crop_right: i64,
    pub crop_top: i64,
    pub display_name: String,
    pub height: i64,
    pub id: String,
    pub preserve_aspect_ratio: bool,
    pub source_id: String,
    pub source_height: Option<i64>,
    pub source_kind: String,
    pub source_width: Option<i64>,
    pub width: i64,
    pub x: i64,
    pub y: i64,
    pub z_index: i64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RecorderSceneRow {
    pub canvas_height: i64,
    pub canvas_width: i64,
    pub desktop_audio_source_id: Option<String>,
    pub id: String,
    pub layout_mode: String,
    pub microphone_audio_source_id: Option<String>,
    pub name: String,
    pub sources: Vec<RecorderSceneSourceRow>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RecorderSceneSourceRow {
    pub crop_bottom: i64,
    pub crop_left: i64,
    pub crop_right: i64,
    pub crop_top: i64,
    pub display_name: String,
    pub height: i64,
    pub id: String,
    pub preserve_aspect_ratio: bool,
    pub source_id: String,
    pub source_height: Option<i64>,
    pub source_kind: String,
    pub source_width: Option<i64>,
    pub width: i64,
    pub x: i64,
    pub y: i64,
    pub z_index: i64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CreateRecordingSegmentInput {
    pub file_path: String,
    pub id: String,
    pub recording_id: String,
    pub segment_index: i64,
    pub started_at_ms: i64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FinalizeRecordingSegmentInput {
    pub desktop_audio_stream_index: Option<i64>,
    pub duration_ms: i64,
    pub ended_at_ms: i64,
    pub id: String,
    pub microphone_audio_stream_index: Option<i64>,
    pub updated_at_ms: i64,
    pub video_stream_index: Option<i64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FailRecordingSegmentInput {
    pub ended_at_ms: i64,
    pub error_code: String,
    pub ffmpeg_exit_code: Option<i64>,
    pub id: String,
    pub updated_at_ms: i64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RecordingSegmentRow {
    pub desktop_audio_stream_index: Option<i64>,
    pub duration_ms: Option<i64>,
    pub ended_at_ms: Option<i64>,
    pub error_code: Option<String>,
    pub ffmpeg_exit_code: Option<i64>,
    pub file_path: String,
    pub id: String,
    pub microphone_audio_stream_index: Option<i64>,
    pub recording_id: String,
    pub segment_index: i64,
    pub started_at_ms: i64,
    pub state: String,
    pub video_stream_index: Option<i64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PortalRestoreTokenRow {
    pub created_at_ms: i64,
    pub id: String,
    pub last_used_at_ms: Option<i64>,
    pub portal: String,
    pub token: String,
    pub updated_at_ms: i64,
}

pub fn create_recording(
    connection: &Connection,
    input: &CreateRecordingInput,
) -> rusqlite::Result<RecordingRow> {
    connection.execute(
        r#"
        INSERT INTO recordings (
            id,
            active_scene_id,
            created_at_ms,
            default_fps,
            notes,
            output_root,
            quality_label,
            status,
            storage_dir
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'planned', ?8)
        "#,
        params![
            input.id,
            input.active_scene_id,
            input.created_at_ms,
            input.default_fps,
            input.notes,
            input.output_root,
            input.quality_label,
            input.storage_dir,
        ],
    )?;

    get_recording(connection, &input.id)
}

pub fn get_recording(connection: &Connection, id: &str) -> rusqlite::Result<RecordingRow> {
    connection.query_row(
        r#"
        SELECT
            id,
            active_scene_id,
            ended_at_ms,
            output_root,
            quality_label,
            started_at_ms,
            status,
            storage_dir
        FROM recordings
        WHERE id = ?1
        "#,
        [id],
        |row| {
            Ok(RecordingRow {
                id: row.get(0)?,
                active_scene_id: row.get(1)?,
                ended_at_ms: row.get(2)?,
                output_root: row.get(3)?,
                quality_label: row.get(4)?,
                started_at_ms: row.get(5)?,
                status: row.get(6)?,
                storage_dir: row.get(7)?,
            })
        },
    )
}

pub fn mark_recording_started(
    connection: &Connection,
    id: &str,
    started_at_ms: i64,
) -> rusqlite::Result<RecordingRow> {
    connection.execute(
        r#"
        UPDATE recordings
        SET
            status = 'recording',
            started_at_ms = ?2,
            ended_at_ms = NULL
        WHERE id = ?1
        "#,
        params![id, started_at_ms],
    )?;

    get_recording(connection, id)
}

pub fn update_recording_status(
    connection: &Connection,
    id: &str,
    status: &str,
) -> rusqlite::Result<RecordingRow> {
    connection.execute(
        "UPDATE recordings SET status = ?2 WHERE id = ?1",
        params![id, status],
    )?;

    get_recording(connection, id)
}

pub fn mark_recording_completed(
    connection: &Connection,
    id: &str,
    ended_at_ms: i64,
) -> rusqlite::Result<RecordingRow> {
    connection.execute(
        r#"
        UPDATE recordings
        SET
            status = 'completed',
            ended_at_ms = ?2
        WHERE id = ?1
        "#,
        params![id, ended_at_ms],
    )?;

    get_recording(connection, id)
}

pub fn save_recorder_scene(
    connection: &mut Connection,
    input: &SaveRecorderSceneInput,
) -> rusqlite::Result<RecorderSceneRow> {
    let transaction = connection.transaction()?;

    transaction.execute(
        r#"
        INSERT INTO recorder_scenes (
            id,
            name,
            canvas_width,
            canvas_height,
            layout_mode,
            desktop_audio_source_id,
            microphone_audio_source_id,
            created_at_ms,
            updated_at_ms
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)
        ON CONFLICT(id) DO UPDATE SET
            name = excluded.name,
            canvas_width = excluded.canvas_width,
            canvas_height = excluded.canvas_height,
            layout_mode = excluded.layout_mode,
            desktop_audio_source_id = excluded.desktop_audio_source_id,
            microphone_audio_source_id = excluded.microphone_audio_source_id,
            updated_at_ms = excluded.updated_at_ms
        "#,
        params![
            input.id,
            input.name,
            input.canvas_width,
            input.canvas_height,
            input.layout_mode,
            input.desktop_audio_source_id,
            input.microphone_audio_source_id,
            input.timestamp_ms,
        ],
    )?;

    transaction.execute(
        "DELETE FROM recorder_scene_sources WHERE scene_id = ?1",
        [&input.id],
    )?;

    for source in &input.sources {
        transaction.execute(
            r#"
            INSERT INTO recorder_scene_sources (
                id,
                scene_id,
                source_id,
                source_kind,
                display_name,
                x,
                y,
                width,
                height,
                source_width,
                source_height,
                crop_left,
                crop_top,
                crop_right,
                crop_bottom,
                z_index,
                preserve_aspect_ratio,
                created_at_ms,
                updated_at_ms
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?18)
            "#,
            params![
                source.id,
                input.id,
                source.source_id,
                source.source_kind,
                source.display_name,
                source.x,
                source.y,
                source.width,
                source.height,
                source.source_width,
                source.source_height,
                source.crop_left,
                source.crop_top,
                source.crop_right,
                source.crop_bottom,
                source.z_index,
                source.preserve_aspect_ratio,
                input.timestamp_ms,
            ],
        )?;
    }

    transaction.commit()?;

    get_recorder_scene(connection, &input.id)
}

pub fn get_recorder_scene(connection: &Connection, id: &str) -> rusqlite::Result<RecorderSceneRow> {
    let mut scene = connection.query_row(
        r#"
        SELECT
            id,
            name,
            canvas_width,
            canvas_height,
            layout_mode,
            desktop_audio_source_id,
            microphone_audio_source_id
        FROM recorder_scenes
        WHERE id = ?1
        "#,
        [id],
        |row| {
            Ok(RecorderSceneRow {
                id: row.get(0)?,
                name: row.get(1)?,
                canvas_width: row.get(2)?,
                canvas_height: row.get(3)?,
                layout_mode: row.get(4)?,
                desktop_audio_source_id: row.get(5)?,
                microphone_audio_source_id: row.get(6)?,
                sources: Vec::new(),
            })
        },
    )?;

    scene.sources = list_recorder_scene_sources(connection, id)?;

    Ok(scene)
}

pub fn get_latest_recorder_scene(
    connection: &Connection,
) -> rusqlite::Result<Option<RecorderSceneRow>> {
    let scene = connection
        .query_row(
            r#"
            SELECT
                id,
                name,
                canvas_width,
                canvas_height,
                layout_mode,
                desktop_audio_source_id,
                microphone_audio_source_id
            FROM recorder_scenes
            ORDER BY updated_at_ms DESC, id ASC
            LIMIT 1
            "#,
            [],
            |row| {
                Ok(RecorderSceneRow {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    canvas_width: row.get(2)?,
                    canvas_height: row.get(3)?,
                    layout_mode: row.get(4)?,
                    desktop_audio_source_id: row.get(5)?,
                    microphone_audio_source_id: row.get(6)?,
                    sources: Vec::new(),
                })
            },
        )
        .optional()?;

    scene
        .map(|mut scene| {
            scene.sources = list_recorder_scene_sources(connection, &scene.id)?;
            Ok(scene)
        })
        .transpose()
}

pub fn create_recording_segment(
    connection: &Connection,
    input: &CreateRecordingSegmentInput,
) -> rusqlite::Result<RecordingSegmentRow> {
    connection.execute(
        r#"
        INSERT INTO recording_segments (
            id,
            recording_id,
            segment_index,
            file_path,
            started_at_ms,
            state,
            created_at_ms,
            updated_at_ms
        ) VALUES (?1, ?2, ?3, ?4, ?5, 'recording', ?5, ?5)
        "#,
        params![
            input.id,
            input.recording_id,
            input.segment_index,
            input.file_path,
            input.started_at_ms,
        ],
    )?;

    get_recording_segment(connection, &input.id)
}

pub fn finalize_recording_segment(
    connection: &Connection,
    input: &FinalizeRecordingSegmentInput,
) -> rusqlite::Result<RecordingSegmentRow> {
    connection.execute(
        r#"
        UPDATE recording_segments
        SET
            ended_at_ms = ?2,
            duration_ms = ?3,
            state = 'finalized',
            video_stream_index = ?4,
            desktop_audio_stream_index = ?5,
            microphone_audio_stream_index = ?6,
            ffmpeg_exit_code = 0,
            error_code = NULL,
            updated_at_ms = ?7
        WHERE id = ?1
        "#,
        params![
            input.id,
            input.ended_at_ms,
            input.duration_ms,
            input.video_stream_index,
            input.desktop_audio_stream_index,
            input.microphone_audio_stream_index,
            input.updated_at_ms,
        ],
    )?;

    get_recording_segment(connection, &input.id)
}

pub fn fail_recording_segment(
    connection: &Connection,
    input: &FailRecordingSegmentInput,
) -> rusqlite::Result<RecordingSegmentRow> {
    connection.execute(
        r#"
        UPDATE recording_segments
        SET
            ended_at_ms = ?2,
            duration_ms = CASE
                WHEN ?2 >= started_at_ms THEN ?2 - started_at_ms
                ELSE NULL
            END,
            state = 'failed',
            ffmpeg_exit_code = ?3,
            error_code = ?4,
            updated_at_ms = ?5
        WHERE id = ?1
        "#,
        params![
            input.id,
            input.ended_at_ms,
            input.ffmpeg_exit_code,
            input.error_code,
            input.updated_at_ms,
        ],
    )?;

    get_recording_segment(connection, &input.id)
}

pub fn list_recording_segments(
    connection: &Connection,
    recording_id: &str,
) -> rusqlite::Result<Vec<RecordingSegmentRow>> {
    let mut statement = connection.prepare(
        r#"
        SELECT
            id,
            recording_id,
            segment_index,
            file_path,
            started_at_ms,
            ended_at_ms,
            duration_ms,
            state,
            video_stream_index,
            desktop_audio_stream_index,
            microphone_audio_stream_index,
            ffmpeg_exit_code,
            error_code
        FROM recording_segments
        WHERE recording_id = ?1
        ORDER BY segment_index ASC
        "#,
    )?;

    let rows = statement.query_map([recording_id], recording_segment_from_row)?;

    rows.collect()
}

pub fn get_portal_restore_token(
    connection: &Connection,
    id: &str,
) -> rusqlite::Result<Option<PortalRestoreTokenRow>> {
    match connection.query_row(
        r#"
        SELECT
            id,
            portal,
            token,
            created_at_ms,
            updated_at_ms,
            last_used_at_ms
        FROM recorder_portal_restore_tokens
        WHERE id = ?1
        "#,
        [id],
        portal_restore_token_from_row,
    ) {
        Ok(row) => Ok(Some(row)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(error) => Err(error),
    }
}

pub fn upsert_portal_restore_token(
    connection: &Connection,
    id: &str,
    portal: &str,
    token: &str,
    timestamp_ms: i64,
) -> rusqlite::Result<PortalRestoreTokenRow> {
    connection.execute(
        r#"
        INSERT INTO recorder_portal_restore_tokens (
            id,
            portal,
            token,
            created_at_ms,
            updated_at_ms,
            last_used_at_ms
        ) VALUES (?1, ?2, ?3, ?4, ?4, ?4)
        ON CONFLICT(id) DO UPDATE SET
            portal = excluded.portal,
            token = excluded.token,
            updated_at_ms = excluded.updated_at_ms,
            last_used_at_ms = excluded.last_used_at_ms
        "#,
        params![id, portal, token, timestamp_ms],
    )?;

    get_portal_restore_token(connection, id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)
}

pub fn mark_portal_restore_token_used(
    connection: &Connection,
    id: &str,
    timestamp_ms: i64,
) -> rusqlite::Result<()> {
    connection.execute(
        r#"
        UPDATE recorder_portal_restore_tokens
        SET last_used_at_ms = ?2
        WHERE id = ?1
        "#,
        params![id, timestamp_ms],
    )?;

    Ok(())
}

fn open_connection(path: &Path) -> rusqlite::Result<Connection> {
    let connection = Connection::open(path)?;
    enable_connection_pragmas(&connection)?;

    Ok(connection)
}

fn enable_connection_pragmas(connection: &Connection) -> rusqlite::Result<()> {
    connection.pragma_update(None, "foreign_keys", "ON")?;
    connection.pragma_update(None, "journal_mode", "WAL")?;
    connection.pragma_update(None, "synchronous", "NORMAL")?;

    Ok(())
}

fn apply_migrations(connection: &mut Connection) -> rusqlite_migration::Result<()> {
    migrations().to_latest(connection)
}

fn collect_database_diagnostics_from_connection(
    connection: &Connection,
) -> rusqlite::Result<(i64, BTreeMap<String, i64>)> {
    let schema_version = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    let mut table_counts = BTreeMap::new();

    for table in CORE_TABLES {
        table_counts.insert(table.to_string(), count_table_rows(connection, table)?);
    }

    Ok((schema_version, table_counts))
}

fn count_table_rows(connection: &Connection, table: &str) -> rusqlite::Result<i64> {
    let statement = format!("SELECT COUNT(*) FROM {table}");

    connection.query_row(&statement, [], |row| row.get(0))
}

fn list_recorder_scene_sources(
    connection: &Connection,
    scene_id: &str,
) -> rusqlite::Result<Vec<RecorderSceneSourceRow>> {
    let mut statement = connection.prepare(
        r#"
        SELECT
            id,
            source_id,
            source_kind,
            display_name,
            x,
            y,
            width,
            height,
            source_width,
            source_height,
            crop_left,
            crop_top,
            crop_right,
            crop_bottom,
            z_index,
            preserve_aspect_ratio
        FROM recorder_scene_sources
        WHERE scene_id = ?1
        ORDER BY z_index ASC, id ASC
        "#,
    )?;

    let rows = statement.query_map([scene_id], |row| {
        Ok(RecorderSceneSourceRow {
            id: row.get(0)?,
            source_id: row.get(1)?,
            source_kind: row.get(2)?,
            display_name: row.get(3)?,
            x: row.get(4)?,
            y: row.get(5)?,
            width: row.get(6)?,
            height: row.get(7)?,
            source_width: row.get(8)?,
            source_height: row.get(9)?,
            crop_left: row.get(10)?,
            crop_top: row.get(11)?,
            crop_right: row.get(12)?,
            crop_bottom: row.get(13)?,
            z_index: row.get(14)?,
            preserve_aspect_ratio: row.get(15)?,
        })
    })?;

    rows.collect()
}

fn get_recording_segment(
    connection: &Connection,
    id: &str,
) -> rusqlite::Result<RecordingSegmentRow> {
    connection.query_row(
        r#"
        SELECT
            id,
            recording_id,
            segment_index,
            file_path,
            started_at_ms,
            ended_at_ms,
            duration_ms,
            state,
            video_stream_index,
            desktop_audio_stream_index,
            microphone_audio_stream_index,
            ffmpeg_exit_code,
            error_code
        FROM recording_segments
        WHERE id = ?1
        "#,
        [id],
        recording_segment_from_row,
    )
}

fn recording_segment_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RecordingSegmentRow> {
    Ok(RecordingSegmentRow {
        id: row.get(0)?,
        recording_id: row.get(1)?,
        segment_index: row.get(2)?,
        file_path: row.get(3)?,
        started_at_ms: row.get(4)?,
        ended_at_ms: row.get(5)?,
        duration_ms: row.get(6)?,
        state: row.get(7)?,
        video_stream_index: row.get(8)?,
        desktop_audio_stream_index: row.get(9)?,
        microphone_audio_stream_index: row.get(10)?,
        ffmpeg_exit_code: row.get(11)?,
        error_code: row.get(12)?,
    })
}

fn portal_restore_token_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<PortalRestoreTokenRow> {
    Ok(PortalRestoreTokenRow {
        id: row.get(0)?,
        portal: row.get(1)?,
        token: row.get(2)?,
        created_at_ms: row.get(3)?,
        updated_at_ms: row.get(4)?,
        last_used_at_ms: row.get(5)?,
    })
}

fn migrations() -> Migrations<'static> {
    Migrations::new(vec![
        M::up(
            r#"
        CREATE TABLE recordings (
            id TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36),
            created_at_ms INTEGER NOT NULL CHECK (created_at_ms >= 0),
            started_at_ms INTEGER CHECK (started_at_ms IS NULL OR started_at_ms >= 0),
            ended_at_ms INTEGER CHECK (
                ended_at_ms IS NULL
                OR (
                    ended_at_ms >= 0
                    AND (started_at_ms IS NULL OR ended_at_ms >= started_at_ms)
                )
            ),
            status TEXT NOT NULL CHECK (status IN ('planned', 'recording', 'paused', 'completed', 'failed', 'deleted')),
            default_fps REAL NOT NULL CHECK (default_fps > 0),
            quality_label TEXT NOT NULL CHECK (quality_label IN ('draft', 'standard', 'high', 'ocr')),
            storage_dir TEXT NOT NULL,
            primary_video_path TEXT,
            notes TEXT
        );

        CREATE TABLE recording_tracks (
            id TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36),
            recording_id TEXT NOT NULL CHECK (length(recording_id) = 36),
            kind TEXT NOT NULL CHECK (kind IN ('screen', 'desktop-audio', 'microphone')),
            label TEXT NOT NULL,
            source_id TEXT,
            path TEXT,
            codec TEXT,
            created_at_ms INTEGER NOT NULL CHECK (created_at_ms >= 0),
            FOREIGN KEY (recording_id) REFERENCES recordings(id) ON DELETE CASCADE
        );

        CREATE TABLE redactions (
            id TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36),
            recording_id TEXT NOT NULL CHECK (length(recording_id) = 36),
            start_offset_ms INTEGER NOT NULL CHECK (start_offset_ms >= 0),
            end_offset_ms INTEGER NOT NULL CHECK (end_offset_ms > start_offset_ms),
            reason TEXT,
            created_at_ms INTEGER NOT NULL CHECK (created_at_ms >= 0),
            FOREIGN KEY (recording_id) REFERENCES recordings(id) ON DELETE CASCADE
        );

        CREATE TABLE capture_events (
            id TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36),
            recording_id TEXT NOT NULL CHECK (length(recording_id) = 36),
            occurred_at_ms INTEGER NOT NULL CHECK (occurred_at_ms >= 0),
            offset_ms INTEGER NOT NULL CHECK (offset_ms >= 0),
            kind TEXT NOT NULL CHECK (kind IN ('keyboard-activity', 'mouse-activity', 'focused-window', 'open-windows', 'capture-state')),
            payload_json TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(payload_json)),
            FOREIGN KEY (recording_id) REFERENCES recordings(id) ON DELETE CASCADE
        );

        CREATE TABLE processing_jobs (
            id TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36),
            recording_id TEXT NOT NULL CHECK (length(recording_id) = 36),
            kind TEXT NOT NULL CHECK (kind IN ('chunking', 'frame-extraction', 'ocr', 'transcription', 'vlm', 'embedding', 'digest')),
            status TEXT NOT NULL CHECK (status IN ('queued', 'running', 'succeeded', 'failed', 'cancelled')),
            created_at_ms INTEGER NOT NULL CHECK (created_at_ms >= 0),
            updated_at_ms INTEGER NOT NULL CHECK (updated_at_ms >= created_at_ms),
            error TEXT,
            FOREIGN KEY (recording_id) REFERENCES recordings(id) ON DELETE CASCADE
        );

        CREATE INDEX idx_recording_tracks_recording_id ON recording_tracks(recording_id);
        CREATE INDEX idx_redactions_recording_interval ON redactions(recording_id, start_offset_ms, end_offset_ms);
        CREATE INDEX idx_capture_events_recording_offset ON capture_events(recording_id, offset_ms);
        CREATE INDEX idx_capture_events_kind ON capture_events(kind);
        CREATE INDEX idx_processing_jobs_recording_status ON processing_jobs(recording_id, status);
        CREATE INDEX idx_processing_jobs_kind_status ON processing_jobs(kind, status);
        "#,
        ),
        M::up(
            r#"
        CREATE TABLE recorder_scenes (
            id TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36),
            name TEXT NOT NULL,
            canvas_width INTEGER NOT NULL CHECK (canvas_width > 0),
            canvas_height INTEGER NOT NULL CHECK (canvas_height > 0),
            layout_mode TEXT NOT NULL CHECK (layout_mode IN ('auto-16-9', 'custom', 'horizontal', 'vertical')),
            created_at_ms INTEGER NOT NULL CHECK (created_at_ms >= 0),
            updated_at_ms INTEGER NOT NULL CHECK (updated_at_ms >= created_at_ms)
        );

        CREATE TABLE recordings_v2 (
            id TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36),
            active_scene_id TEXT CHECK (active_scene_id IS NULL OR length(active_scene_id) = 36),
            created_at_ms INTEGER NOT NULL CHECK (created_at_ms >= 0),
            started_at_ms INTEGER CHECK (started_at_ms IS NULL OR started_at_ms >= 0),
            ended_at_ms INTEGER CHECK (
                ended_at_ms IS NULL
                OR (
                    ended_at_ms >= 0
                    AND (started_at_ms IS NULL OR ended_at_ms >= started_at_ms)
                )
            ),
            status TEXT NOT NULL CHECK (status IN ('planned', 'recording', 'paused', 'completed', 'failed', 'deleted')),
            default_fps REAL NOT NULL CHECK (default_fps > 0),
            quality_label TEXT NOT NULL CHECK (quality_label IN ('draft', 'standard', 'high', 'ocr', 'balanced-ocr')),
            storage_dir TEXT NOT NULL,
            output_root TEXT,
            primary_video_path TEXT,
            notes TEXT,
            FOREIGN KEY (active_scene_id) REFERENCES recorder_scenes(id) ON DELETE SET NULL
        );

        INSERT INTO recordings_v2 (
            id,
            created_at_ms,
            started_at_ms,
            ended_at_ms,
            status,
            default_fps,
            quality_label,
            storage_dir,
            primary_video_path,
            notes
        )
        SELECT
            id,
            created_at_ms,
            started_at_ms,
            ended_at_ms,
            status,
            default_fps,
            quality_label,
            storage_dir,
            primary_video_path,
            notes
        FROM recordings;

        CREATE TEMP TABLE recording_tracks_v1_backup AS SELECT * FROM recording_tracks;
        CREATE TEMP TABLE redactions_v1_backup AS SELECT * FROM redactions;
        CREATE TEMP TABLE capture_events_v1_backup AS SELECT * FROM capture_events;
        CREATE TEMP TABLE processing_jobs_v1_backup AS SELECT * FROM processing_jobs;

        DROP TABLE recordings;
        ALTER TABLE recordings_v2 RENAME TO recordings;

        INSERT INTO recording_tracks (
            id,
            recording_id,
            kind,
            label,
            source_id,
            path,
            codec,
            created_at_ms
        )
        SELECT
            id,
            recording_id,
            kind,
            label,
            source_id,
            path,
            codec,
            created_at_ms
        FROM recording_tracks_v1_backup;

        INSERT INTO redactions (
            id,
            recording_id,
            start_offset_ms,
            end_offset_ms,
            reason,
            created_at_ms
        )
        SELECT
            id,
            recording_id,
            start_offset_ms,
            end_offset_ms,
            reason,
            created_at_ms
        FROM redactions_v1_backup;

        INSERT INTO capture_events (
            id,
            recording_id,
            occurred_at_ms,
            offset_ms,
            kind,
            payload_json
        )
        SELECT
            id,
            recording_id,
            occurred_at_ms,
            offset_ms,
            kind,
            payload_json
        FROM capture_events_v1_backup;

        INSERT INTO processing_jobs (
            id,
            recording_id,
            kind,
            status,
            created_at_ms,
            updated_at_ms,
            error
        )
        SELECT
            id,
            recording_id,
            kind,
            status,
            created_at_ms,
            updated_at_ms,
            error
        FROM processing_jobs_v1_backup;

        DROP TABLE recording_tracks_v1_backup;
        DROP TABLE redactions_v1_backup;
        DROP TABLE capture_events_v1_backup;
        DROP TABLE processing_jobs_v1_backup;

        CREATE TABLE recording_segments (
            id TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36),
            recording_id TEXT NOT NULL CHECK (length(recording_id) = 36),
            segment_index INTEGER NOT NULL CHECK (segment_index > 0),
            file_path TEXT NOT NULL,
            started_at_ms INTEGER NOT NULL CHECK (started_at_ms >= 0),
            ended_at_ms INTEGER CHECK (
                ended_at_ms IS NULL
                OR (
                    ended_at_ms >= 0
                    AND ended_at_ms >= started_at_ms
                )
            ),
            duration_ms INTEGER CHECK (duration_ms IS NULL OR duration_ms >= 0),
            state TEXT NOT NULL CHECK (state IN ('recording', 'finalized', 'failed')),
            video_stream_index INTEGER CHECK (video_stream_index IS NULL OR video_stream_index >= 0),
            desktop_audio_stream_index INTEGER CHECK (desktop_audio_stream_index IS NULL OR desktop_audio_stream_index >= 0),
            microphone_audio_stream_index INTEGER CHECK (microphone_audio_stream_index IS NULL OR microphone_audio_stream_index >= 0),
            ffmpeg_exit_code INTEGER,
            error_code TEXT,
            created_at_ms INTEGER NOT NULL CHECK (created_at_ms >= 0),
            updated_at_ms INTEGER NOT NULL CHECK (updated_at_ms >= created_at_ms),
            UNIQUE (recording_id, segment_index),
            FOREIGN KEY (recording_id) REFERENCES recordings(id) ON DELETE CASCADE
        );

        CREATE TABLE recorder_scene_sources (
            id TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36),
            scene_id TEXT NOT NULL CHECK (length(scene_id) = 36),
            source_id TEXT NOT NULL,
            source_kind TEXT NOT NULL CHECK (source_kind IN ('monitor')),
            display_name TEXT NOT NULL,
            x INTEGER NOT NULL,
            y INTEGER NOT NULL,
            width INTEGER NOT NULL CHECK (width > 0),
            height INTEGER NOT NULL CHECK (height > 0),
            crop_left INTEGER NOT NULL CHECK (crop_left >= 0),
            crop_top INTEGER NOT NULL CHECK (crop_top >= 0),
            crop_right INTEGER NOT NULL CHECK (crop_right >= 0),
            crop_bottom INTEGER NOT NULL CHECK (crop_bottom >= 0),
            z_index INTEGER NOT NULL,
            preserve_aspect_ratio INTEGER NOT NULL CHECK (preserve_aspect_ratio IN (0, 1)),
            created_at_ms INTEGER NOT NULL CHECK (created_at_ms >= 0),
            updated_at_ms INTEGER NOT NULL CHECK (updated_at_ms >= created_at_ms),
            FOREIGN KEY (scene_id) REFERENCES recorder_scenes(id) ON DELETE CASCADE
        );

        CREATE INDEX idx_recordings_active_scene_id ON recordings(active_scene_id);
        CREATE INDEX idx_recording_segments_recording_id ON recording_segments(recording_id);
        CREATE INDEX idx_recorder_scene_sources_scene_z ON recorder_scene_sources(scene_id, z_index);
        "#,
        ),
        M::up(
            r#"
        CREATE TABLE recorder_portal_restore_tokens (
            id TEXT PRIMARY KEY NOT NULL,
            portal TEXT NOT NULL CHECK (portal IN ('linux-screencast')),
            token TEXT NOT NULL CHECK (length(token) > 0),
            created_at_ms INTEGER NOT NULL CHECK (created_at_ms >= 0),
            updated_at_ms INTEGER NOT NULL CHECK (updated_at_ms >= created_at_ms),
            last_used_at_ms INTEGER CHECK (last_used_at_ms IS NULL OR last_used_at_ms >= created_at_ms)
        );
        "#,
        ),
        M::up(
            r#"
        ALTER TABLE recorder_scene_sources
            ADD COLUMN source_width INTEGER CHECK (source_width IS NULL OR source_width > 0);
        ALTER TABLE recorder_scene_sources
            ADD COLUMN source_height INTEGER CHECK (source_height IS NULL OR source_height > 0);
        "#,
        ),
        M::up(
            r#"
        -- Existing alpha databases have recorder_scenes referenced by child
        -- tables, and this migration runner wraps each migration in a
        -- transaction. Patch only the CHECK expression instead of rebuilding
        -- the foreign-key-linked table with PRAGMA foreign_keys disabled.
        PRAGMA writable_schema = ON;

        UPDATE sqlite_schema
        SET sql = replace(
            sql,
            'layout_mode TEXT NOT NULL CHECK (layout_mode IN (''horizontal'', ''custom''))',
            'layout_mode TEXT NOT NULL CHECK (layout_mode IN (''auto-16-9'', ''custom'', ''horizontal'', ''vertical''))'
        )
        WHERE type = 'table'
            AND name = 'recorder_scenes'
            AND sql LIKE '%layout_mode TEXT NOT NULL CHECK (layout_mode IN (''horizontal'', ''custom''))%';

        PRAGMA schema_version = 5000;
        PRAGMA writable_schema = OFF;
        "#,
        ),
        M::up(
            r#"
        ALTER TABLE recorder_scenes
            ADD COLUMN desktop_audio_source_id TEXT;
        ALTER TABLE recorder_scenes
            ADD COLUMN microphone_audio_source_id TEXT;
        "#,
        ),
    ])
}

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

#[cfg(test)]
mod tests {
    use std::{path::PathBuf, sync::Mutex};

    use rusqlite::Connection;
    use uuid::Uuid;

    use super::{
        apply_migrations, collect_database_diagnostics_from_connection, create_recording,
        create_recording_segment, enable_connection_pragmas, fail_recording_segment,
        finalize_recording_segment, get_latest_recorder_scene, get_recorder_scene, get_recording,
        list_recording_segments, mark_portal_restore_token_used, migrations, save_recorder_scene,
        upsert_portal_restore_token, AppStorage, CreateRecordingInput, CreateRecordingSegmentInput,
        FailRecordingSegmentInput, FinalizeRecordingSegmentInput, SaveRecorderSceneInput,
        SaveRecorderSceneSourceInput,
    };

    fn migrated_memory_connection() -> Connection {
        let mut connection = Connection::open_in_memory().expect("open in-memory sqlite database");
        enable_connection_pragmas(&connection).expect("enable sqlite pragmas");
        apply_migrations(&mut connection).expect("apply migrations");

        connection
    }

    #[test]
    fn migration_creates_core_tables_and_schema_version() {
        let connection = migrated_memory_connection();
        let (schema_version, table_counts) =
            collect_database_diagnostics_from_connection(&connection)
                .expect("collect database diagnostics");

        assert_eq!(schema_version, 6);
        assert_eq!(table_counts["recordings"], 0);
        assert_eq!(table_counts["recording_tracks"], 0);
        assert_eq!(table_counts["recording_segments"], 0);
        assert_eq!(table_counts["recorder_scenes"], 0);
        assert_eq!(table_counts["recorder_scene_sources"], 0);
        assert_eq!(table_counts["recorder_portal_restore_tokens"], 0);
        assert_eq!(table_counts["redactions"], 0);
        assert_eq!(table_counts["capture_events"], 0);
        assert_eq!(table_counts["processing_jobs"], 0);
    }

    #[test]
    fn migration_enables_foreign_keys() {
        let connection = migrated_memory_connection();
        let enabled: i64 = connection
            .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
            .expect("query foreign_keys pragma");

        assert_eq!(enabled, 1);
    }

    #[test]
    fn recording_status_constraint_rejects_unknown_values() {
        let connection = migrated_memory_connection();
        let id = Uuid::now_v7().to_string();

        let error = connection
            .execute(
                r#"
                INSERT INTO recordings (
                    id,
                    created_at_ms,
                    status,
                    default_fps,
                    quality_label,
                    storage_dir
                ) VALUES (?1, 1, 'unknown', 1.0, 'ocr', '/tmp/daydream')
                "#,
                [&id],
            )
            .expect_err("unknown recording status should fail");

        assert!(error.to_string().contains("CHECK constraint failed"));
    }

    #[test]
    fn redaction_interval_must_have_positive_duration() {
        let connection = migrated_memory_connection();
        let recording_id = Uuid::now_v7().to_string();

        connection
            .execute(
                r#"
                INSERT INTO recordings (
                    id,
                    created_at_ms,
                    status,
                    default_fps,
                    quality_label,
                    storage_dir
                ) VALUES (?1, 1, 'completed', 1.0, 'ocr', '/tmp/daydream')
                "#,
                [&recording_id],
            )
            .expect("insert recording");

        let error = connection
            .execute(
                r#"
                INSERT INTO redactions (
                    id,
                    recording_id,
                    start_offset_ms,
                    end_offset_ms,
                    created_at_ms
                ) VALUES (?1, ?2, 10, 10, 1)
                "#,
                [&Uuid::now_v7().to_string(), &recording_id],
            )
            .expect_err("zero-length redaction should fail");

        assert!(error.to_string().contains("CHECK constraint failed"));
    }

    #[test]
    fn track_requires_existing_recording() {
        let connection = migrated_memory_connection();

        let error = connection
            .execute(
                r#"
                INSERT INTO recording_tracks (
                    id,
                    recording_id,
                    kind,
                    label,
                    created_at_ms
                ) VALUES (?1, ?2, 'screen', 'Display 1', 1)
                "#,
                [&Uuid::now_v7().to_string(), &Uuid::now_v7().to_string()],
            )
            .expect_err("track without recording should fail");

        assert!(error.to_string().contains("FOREIGN KEY constraint failed"));
    }

    #[test]
    fn existing_v1_database_migrates_to_v2_without_losing_recordings() {
        let mut connection = Connection::open_in_memory().expect("open in-memory sqlite database");
        enable_connection_pragmas(&connection).expect("enable sqlite pragmas");
        migrations()
            .to_version(&mut connection, 1)
            .expect("apply v1 migration");

        let recording_id = Uuid::now_v7().to_string();
        connection
            .execute(
                r#"
                INSERT INTO recordings (
                    id,
                    created_at_ms,
                    status,
                    default_fps,
                    quality_label,
                    storage_dir
                ) VALUES (?1, 1, 'planned', 1.0, 'ocr', '/tmp/daydream')
                "#,
                [&recording_id],
            )
            .expect("insert v1 recording");

        apply_migrations(&mut connection).expect("apply remaining migrations");

        let (schema_version, table_counts) =
            collect_database_diagnostics_from_connection(&connection)
                .expect("collect database diagnostics");
        let migrated = get_recording(&connection, &recording_id).expect("read migrated recording");

        assert_eq!(schema_version, 6);
        assert_eq!(table_counts["recordings"], 1);
        assert_eq!(migrated.quality_label, "ocr");
        assert_eq!(migrated.active_scene_id, None);
        assert_eq!(migrated.output_root, None);
    }

    #[test]
    fn existing_v1_database_with_child_rows_migrates_to_v2() {
        let mut connection = Connection::open_in_memory().expect("open in-memory sqlite database");
        enable_connection_pragmas(&connection).expect("enable sqlite pragmas");
        migrations()
            .to_version(&mut connection, 1)
            .expect("apply v1 migration");

        let recording_id = Uuid::now_v7().to_string();
        connection
            .execute(
                r#"
                INSERT INTO recordings (
                    id,
                    created_at_ms,
                    status,
                    default_fps,
                    quality_label,
                    storage_dir
                ) VALUES (?1, 1, 'planned', 1.0, 'ocr', '/tmp/daydream')
                "#,
                [&recording_id],
            )
            .expect("insert v1 recording");
        connection
            .execute(
                r#"
                INSERT INTO recording_tracks (
                    id,
                    recording_id,
                    kind,
                    label,
                    created_at_ms
                ) VALUES (?1, ?2, 'screen', 'Display 1', 1)
                "#,
                [&Uuid::now_v7().to_string(), &recording_id],
            )
            .expect("insert v1 track");

        apply_migrations(&mut connection).expect("apply remaining migrations");

        let track_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM recording_tracks", [], |row| {
                row.get(0)
            })
            .expect("count tracks");

        assert_eq!(track_count, 1);
    }

    #[test]
    fn recording_accepts_balanced_ocr_and_minimal_recorder_metadata() {
        let connection = migrated_memory_connection();
        let scene_id = Uuid::now_v7().to_string();
        let recording_id = Uuid::now_v7().to_string();

        connection
            .execute(
                r#"
                INSERT INTO recorder_scenes (
                    id,
                    name,
                    canvas_width,
                    canvas_height,
                    layout_mode,
                    created_at_ms,
                    updated_at_ms
                ) VALUES (?1, 'Scene', 2560, 1440, 'horizontal', 1, 1)
                "#,
                [&scene_id],
            )
            .expect("insert scene");

        let recording = create_recording(
            &connection,
            &CreateRecordingInput {
                id: recording_id.clone(),
                active_scene_id: Some(scene_id.clone()),
                created_at_ms: 2,
                default_fps: 1.0,
                notes: None,
                output_root: Some("/tmp/daydream/recordings".to_string()),
                quality_label: "balanced-ocr".to_string(),
                storage_dir: "/tmp/daydream".to_string(),
            },
        )
        .expect("create recording");

        assert_eq!(recording.id, recording_id);
        assert_eq!(recording.active_scene_id, Some(scene_id.clone()));
        assert_eq!(
            recording.output_root,
            Some("/tmp/daydream/recordings".to_string()),
        );
        assert_eq!(recording.quality_label, "balanced-ocr");

        connection
            .execute("DELETE FROM recorder_scenes WHERE id = ?1", [&scene_id])
            .expect("delete scene");

        let recording = get_recording(&connection, &recording_id).expect("read recording");
        assert_eq!(recording.active_scene_id, None);
    }

    #[test]
    fn scene_helper_persists_absolute_pixel_sources_sorted_by_z_index() {
        let mut connection = migrated_memory_connection();
        let scene_id = Uuid::now_v7().to_string();

        let scene = save_recorder_scene(
            &mut connection,
            &SaveRecorderSceneInput {
                canvas_height: 1440,
                canvas_width: 5120,
                desktop_audio_source_id: Some("desktop-monitor".to_string()),
                id: scene_id.clone(),
                layout_mode: "horizontal".to_string(),
                microphone_audio_source_id: Some("microphone".to_string()),
                name: "Workday".to_string(),
                sources: vec![
                    SaveRecorderSceneSourceInput {
                        crop_bottom: 0,
                        crop_left: 0,
                        crop_right: 0,
                        crop_top: 0,
                        display_name: "Right".to_string(),
                        height: 1440,
                        id: Uuid::now_v7().to_string(),
                        preserve_aspect_ratio: true,
                        source_id: "monitor-2".to_string(),
                        source_height: Some(1440),
                        source_kind: "monitor".to_string(),
                        source_width: Some(2560),
                        width: 2560,
                        x: 2560,
                        y: 0,
                        z_index: 2,
                    },
                    SaveRecorderSceneSourceInput {
                        crop_bottom: 4,
                        crop_left: 1,
                        crop_right: 3,
                        crop_top: 2,
                        display_name: "Left".to_string(),
                        height: 1440,
                        id: Uuid::now_v7().to_string(),
                        preserve_aspect_ratio: false,
                        source_id: "monitor-1".to_string(),
                        source_height: Some(1440),
                        source_kind: "monitor".to_string(),
                        source_width: Some(2560),
                        width: 2560,
                        x: 0,
                        y: 0,
                        z_index: 1,
                    },
                ],
                timestamp_ms: 10,
            },
        )
        .expect("save scene");

        assert_eq!(scene.id, scene_id);
        assert_eq!(scene.canvas_width, 5120);
        assert_eq!(
            scene.desktop_audio_source_id,
            Some("desktop-monitor".to_string()),
        );
        assert_eq!(
            scene.microphone_audio_source_id,
            Some("microphone".to_string()),
        );
        assert_eq!(scene.sources.len(), 2);
        assert_eq!(scene.sources[0].source_id, "monitor-1");
        assert_eq!(scene.sources[0].crop_left, 1);
        assert!(!scene.sources[0].preserve_aspect_ratio);
        assert_eq!(scene.sources[1].source_id, "monitor-2");
    }

    #[test]
    fn saving_existing_scene_replaces_sources_atomically() {
        let mut connection = migrated_memory_connection();
        let scene_id = Uuid::now_v7().to_string();

        save_recorder_scene(
            &mut connection,
            &SaveRecorderSceneInput {
                canvas_height: 1440,
                canvas_width: 2560,
                desktop_audio_source_id: None,
                id: scene_id.clone(),
                layout_mode: "horizontal".to_string(),
                microphone_audio_source_id: None,
                name: "Initial".to_string(),
                sources: vec![scene_source("monitor-1", 0)],
                timestamp_ms: 1,
            },
        )
        .expect("save initial scene");

        save_recorder_scene(
            &mut connection,
            &SaveRecorderSceneInput {
                canvas_height: 1440,
                canvas_width: 2560,
                desktop_audio_source_id: None,
                id: scene_id.clone(),
                layout_mode: "custom".to_string(),
                microphone_audio_source_id: None,
                name: "Updated".to_string(),
                sources: vec![scene_source("monitor-2", 0)],
                timestamp_ms: 2,
            },
        )
        .expect("update scene");

        let scene = get_recorder_scene(&connection, &scene_id).expect("read scene");

        assert_eq!(scene.name, "Updated");
        assert_eq!(scene.layout_mode, "custom");
        assert_eq!(scene.sources.len(), 1);
        assert_eq!(scene.sources[0].source_id, "monitor-2");
    }

    #[test]
    fn scene_helper_accepts_current_layout_modes() {
        let mut connection = migrated_memory_connection();

        for (index, layout_mode) in ["auto-16-9", "custom", "horizontal", "vertical"]
            .into_iter()
            .enumerate()
        {
            let scene_id = Uuid::now_v7().to_string();
            let scene = save_recorder_scene(
                &mut connection,
                &SaveRecorderSceneInput {
                    canvas_height: 1440,
                    canvas_width: 2560,
                    desktop_audio_source_id: None,
                    id: scene_id,
                    layout_mode: layout_mode.to_string(),
                    microphone_audio_source_id: None,
                    name: format!("Layout {layout_mode}"),
                    sources: vec![scene_source("monitor-1", index as i64)],
                    timestamp_ms: index as i64 + 1,
                },
            )
            .expect("save scene with current layout mode");

            assert_eq!(scene.layout_mode, layout_mode);
        }
    }

    #[test]
    fn existing_v4_scene_constraint_migrates_to_current_layout_modes() {
        let mut connection = Connection::open_in_memory().expect("open in-memory sqlite database");
        enable_connection_pragmas(&connection).expect("enable sqlite pragmas");
        create_v4_recorder_scene_tables(&connection);
        let existing_scene_id = Uuid::now_v7().to_string();
        connection
            .execute(
                r#"
                INSERT INTO recorder_scenes (
                    id,
                    name,
                    canvas_width,
                    canvas_height,
                    layout_mode,
                    created_at_ms,
                    updated_at_ms
                ) VALUES (?1, 'Existing scene', 2560, 1440, 'horizontal', 1, 1)
                "#,
                [&existing_scene_id],
            )
            .expect("insert existing v4 scene");
        connection
            .pragma_update(None, "user_version", 4)
            .expect("set v4 schema version");

        apply_migrations(&mut connection).expect("apply current layout migration");

        let existing_scene =
            get_recorder_scene(&connection, &existing_scene_id).expect("read migrated v4 scene");
        assert_eq!(existing_scene.desktop_audio_source_id, None);
        assert_eq!(existing_scene.microphone_audio_source_id, None);

        let scene_id = Uuid::now_v7().to_string();
        let scene = save_recorder_scene(
            &mut connection,
            &SaveRecorderSceneInput {
                canvas_height: 1440,
                canvas_width: 2560,
                desktop_audio_source_id: None,
                id: scene_id,
                layout_mode: "auto-16-9".to_string(),
                microphone_audio_source_id: None,
                name: "Auto scene".to_string(),
                sources: vec![scene_source("monitor-1", 0)],
                timestamp_ms: 1,
            },
        )
        .expect("save scene after layout constraint migration");

        assert_eq!(scene.layout_mode, "auto-16-9");
    }

    #[test]
    fn latest_scene_helper_returns_newest_saved_scene() {
        let mut connection = migrated_memory_connection();
        let older_scene_id = Uuid::now_v7().to_string();
        let newer_scene_id = Uuid::now_v7().to_string();

        save_recorder_scene(
            &mut connection,
            &SaveRecorderSceneInput {
                canvas_height: 720,
                canvas_width: 1280,
                desktop_audio_source_id: None,
                id: older_scene_id,
                layout_mode: "horizontal".to_string(),
                microphone_audio_source_id: None,
                name: "Older".to_string(),
                sources: vec![scene_source("monitor-1", 0)],
                timestamp_ms: 10,
            },
        )
        .expect("save older scene");
        save_recorder_scene(
            &mut connection,
            &SaveRecorderSceneInput {
                canvas_height: 1440,
                canvas_width: 2560,
                desktop_audio_source_id: None,
                id: newer_scene_id.clone(),
                layout_mode: "custom".to_string(),
                microphone_audio_source_id: None,
                name: "Newer".to_string(),
                sources: vec![scene_source("monitor-2", 0)],
                timestamp_ms: 20,
            },
        )
        .expect("save newer scene");

        let latest = get_latest_recorder_scene(&connection)
            .expect("read latest scene")
            .expect("latest scene");

        assert_eq!(latest.id, newer_scene_id);
        assert_eq!(latest.name, "Newer");
        assert_eq!(latest.sources[0].source_id, "monitor-2");
    }

    #[test]
    fn latest_scene_helper_returns_none_without_scenes() {
        let connection = migrated_memory_connection();

        let latest = get_latest_recorder_scene(&connection).expect("read latest scene");

        assert_eq!(latest, None);
    }

    #[test]
    fn app_storage_exposes_mutable_connection_for_scene_transactions() {
        let connection = migrated_memory_connection();
        let scene_id = Uuid::now_v7().to_string();
        let storage = AppStorage {
            connection: Mutex::new(connection),
            path: PathBuf::from("/tmp/daydream.sqlite3"),
        };

        let scene = storage
            .with_connection_mut(|connection| {
                save_recorder_scene(
                    connection,
                    &SaveRecorderSceneInput {
                        canvas_height: 1440,
                        canvas_width: 2560,
                        desktop_audio_source_id: None,
                        id: scene_id.clone(),
                        layout_mode: "horizontal".to_string(),
                        microphone_audio_source_id: None,
                        name: "Managed storage scene".to_string(),
                        sources: vec![scene_source("monitor-1", 0)],
                        timestamp_ms: 1,
                    },
                )
            })
            .expect("save scene through managed storage");

        assert_eq!(scene.id, scene_id);
        assert_eq!(scene.sources.len(), 1);
    }

    #[test]
    fn segment_helpers_create_finalize_fail_and_list_in_order() {
        let connection = migrated_memory_connection();
        let recording_id = create_test_recording(&connection);
        let first_segment_id = Uuid::now_v7().to_string();
        let second_segment_id = Uuid::now_v7().to_string();

        create_recording_segment(
            &connection,
            &CreateRecordingSegmentInput {
                file_path: "/tmp/daydream/0001.mkv".to_string(),
                id: first_segment_id.clone(),
                recording_id: recording_id.clone(),
                segment_index: 1,
                started_at_ms: 10,
            },
        )
        .expect("create first segment");
        create_recording_segment(
            &connection,
            &CreateRecordingSegmentInput {
                file_path: "/tmp/daydream/0002.mkv".to_string(),
                id: second_segment_id.clone(),
                recording_id: recording_id.clone(),
                segment_index: 2,
                started_at_ms: 30,
            },
        )
        .expect("create second segment");

        let finalized = finalize_recording_segment(
            &connection,
            &FinalizeRecordingSegmentInput {
                desktop_audio_stream_index: Some(1),
                duration_ms: 15,
                ended_at_ms: 25,
                id: first_segment_id,
                microphone_audio_stream_index: Some(2),
                updated_at_ms: 26,
                video_stream_index: Some(0),
            },
        )
        .expect("finalize segment");
        let failed = fail_recording_segment(
            &connection,
            &FailRecordingSegmentInput {
                ended_at_ms: 38,
                error_code: "ffmpeg_failed".to_string(),
                ffmpeg_exit_code: Some(1),
                id: second_segment_id,
                updated_at_ms: 39,
            },
        )
        .expect("fail segment");

        let segments = list_recording_segments(&connection, &recording_id).expect("list segments");

        assert_eq!(finalized.state, "finalized");
        assert_eq!(finalized.video_stream_index, Some(0));
        assert_eq!(failed.state, "failed");
        assert_eq!(failed.error_code, Some("ffmpeg_failed".to_string()));
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].segment_index, 1);
        assert_eq!(segments[1].segment_index, 2);
    }

    #[test]
    fn duplicate_segment_index_per_recording_is_rejected() {
        let connection = migrated_memory_connection();
        let recording_id = create_test_recording(&connection);

        create_recording_segment(
            &connection,
            &CreateRecordingSegmentInput {
                file_path: "/tmp/daydream/0001.mkv".to_string(),
                id: Uuid::now_v7().to_string(),
                recording_id: recording_id.clone(),
                segment_index: 1,
                started_at_ms: 10,
            },
        )
        .expect("create first segment");

        let error = create_recording_segment(
            &connection,
            &CreateRecordingSegmentInput {
                file_path: "/tmp/daydream/0001-copy.mkv".to_string(),
                id: Uuid::now_v7().to_string(),
                recording_id,
                segment_index: 1,
                started_at_ms: 11,
            },
        )
        .expect_err("duplicate segment index should fail");

        assert!(error.to_string().contains("UNIQUE constraint failed"));
    }

    #[test]
    fn portal_restore_token_helpers_keep_token_backend_only() {
        let connection = migrated_memory_connection();

        let first = upsert_portal_restore_token(
            &connection,
            "linux-screencast-monitor",
            "linux-screencast",
            "token-1",
            10,
        )
        .expect("insert restore token");

        assert_eq!(first.token, "token-1");
        assert_eq!(first.created_at_ms, 10);
        assert_eq!(first.updated_at_ms, 10);
        assert_eq!(first.last_used_at_ms, Some(10));

        let second = upsert_portal_restore_token(
            &connection,
            "linux-screencast-monitor",
            "linux-screencast",
            "token-2",
            20,
        )
        .expect("update restore token");

        assert_eq!(second.token, "token-2");
        assert_eq!(second.created_at_ms, 10);
        assert_eq!(second.updated_at_ms, 20);
        assert_eq!(second.last_used_at_ms, Some(20));

        mark_portal_restore_token_used(&connection, "linux-screencast-monitor", 30)
            .expect("mark restore token used");

        let restored = super::get_portal_restore_token(&connection, "linux-screencast-monitor")
            .expect("read restore token")
            .expect("restore token exists");

        assert_eq!(restored.token, "token-2");
        assert_eq!(restored.last_used_at_ms, Some(30));
    }

    #[test]
    fn recorder_constraints_reject_invalid_values() {
        let connection = migrated_memory_connection();
        let recording_id = create_test_recording(&connection);
        let scene_id = Uuid::now_v7().to_string();

        let segment_error = connection
            .execute(
                r#"
                INSERT INTO recording_segments (
                    id,
                    recording_id,
                    segment_index,
                    file_path,
                    started_at_ms,
                    ended_at_ms,
                    state,
                    created_at_ms,
                    updated_at_ms
                ) VALUES (?1, ?2, 1, '/tmp/daydream/0001.mkv', 20, 19, 'recording', 20, 20)
                "#,
                [&Uuid::now_v7().to_string(), &recording_id],
            )
            .expect_err("segment end before start should fail");
        assert!(segment_error
            .to_string()
            .contains("CHECK constraint failed"));

        let scene_error = connection
            .execute(
                r#"
                INSERT INTO recorder_scenes (
                    id,
                    name,
                    canvas_width,
                    canvas_height,
                    layout_mode,
                    created_at_ms,
                    updated_at_ms
                ) VALUES (?1, 'Scene', 2560, 1440, 'diagonal', 1, 1)
                "#,
                [&scene_id],
            )
            .expect_err("invalid layout mode should fail");
        assert!(scene_error.to_string().contains("CHECK constraint failed"));

        connection
            .execute(
                r#"
                INSERT INTO recorder_scenes (
                    id,
                    name,
                    canvas_width,
                    canvas_height,
                    layout_mode,
                    created_at_ms,
                    updated_at_ms
                ) VALUES (?1, 'Scene', 2560, 1440, 'horizontal', 1, 1)
                "#,
                [&scene_id],
            )
            .expect("insert scene");

        let source_error = connection
            .execute(
                r#"
                INSERT INTO recorder_scene_sources (
                    id,
                    scene_id,
                    source_id,
                    source_kind,
                    display_name,
                    x,
                    y,
                    width,
                    height,
                    crop_left,
                    crop_top,
                    crop_right,
                    crop_bottom,
                    z_index,
                    preserve_aspect_ratio,
                    created_at_ms,
                    updated_at_ms
                ) VALUES (?1, ?2, 'monitor-1', 'window', 'Display', 0, 0, 0, 1440, 0, 0, 0, 0, 0, 2, 1, 1)
                "#,
                [&Uuid::now_v7().to_string(), &scene_id],
            )
            .expect_err("invalid source kind, size, and boolean should fail");
        assert!(source_error.to_string().contains("CHECK constraint failed"));
    }

    #[test]
    fn recorder_foreign_keys_are_enforced() {
        let connection = migrated_memory_connection();

        let segment_error = create_recording_segment(
            &connection,
            &CreateRecordingSegmentInput {
                file_path: "/tmp/daydream/0001.mkv".to_string(),
                id: Uuid::now_v7().to_string(),
                recording_id: Uuid::now_v7().to_string(),
                segment_index: 1,
                started_at_ms: 10,
            },
        )
        .expect_err("segment without recording should fail");
        assert!(segment_error
            .to_string()
            .contains("FOREIGN KEY constraint failed"));

        let source_error = connection
            .execute(
                r#"
                INSERT INTO recorder_scene_sources (
                    id,
                    scene_id,
                    source_id,
                    source_kind,
                    display_name,
                    x,
                    y,
                    width,
                    height,
                    crop_left,
                    crop_top,
                    crop_right,
                    crop_bottom,
                    z_index,
                    preserve_aspect_ratio,
                    created_at_ms,
                    updated_at_ms
                ) VALUES (?1, ?2, 'monitor-1', 'monitor', 'Display', 0, 0, 2560, 1440, 0, 0, 0, 0, 0, 1, 1, 1)
                "#,
                [&Uuid::now_v7().to_string(), &Uuid::now_v7().to_string()],
            )
            .expect_err("source without scene should fail");
        assert!(source_error
            .to_string()
            .contains("FOREIGN KEY constraint failed"));
    }

    fn create_test_recording(connection: &Connection) -> String {
        let recording_id = Uuid::now_v7().to_string();

        create_recording(
            connection,
            &CreateRecordingInput {
                id: recording_id.clone(),
                active_scene_id: None,
                created_at_ms: 1,
                default_fps: 1.0,
                notes: None,
                output_root: None,
                quality_label: "balanced-ocr".to_string(),
                storage_dir: "/tmp/daydream".to_string(),
            },
        )
        .expect("create recording");

        recording_id
    }

    fn create_v4_recorder_scene_tables(connection: &Connection) {
        connection
            .execute_batch(
                r#"
                CREATE TABLE recorder_scenes (
                    id TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36),
                    name TEXT NOT NULL,
                    canvas_width INTEGER NOT NULL CHECK (canvas_width > 0),
                    canvas_height INTEGER NOT NULL CHECK (canvas_height > 0),
                    layout_mode TEXT NOT NULL CHECK (layout_mode IN ('horizontal', 'custom')),
                    created_at_ms INTEGER NOT NULL CHECK (created_at_ms >= 0),
                    updated_at_ms INTEGER NOT NULL CHECK (updated_at_ms >= created_at_ms)
                );

                CREATE TABLE recorder_scene_sources (
                    id TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36),
                    scene_id TEXT NOT NULL CHECK (length(scene_id) = 36),
                    source_id TEXT NOT NULL,
                    source_kind TEXT NOT NULL CHECK (source_kind IN ('monitor')),
                    display_name TEXT NOT NULL,
                    x INTEGER NOT NULL,
                    y INTEGER NOT NULL,
                    width INTEGER NOT NULL CHECK (width > 0),
                    height INTEGER NOT NULL CHECK (height > 0),
                    source_width INTEGER CHECK (source_width IS NULL OR source_width > 0),
                    source_height INTEGER CHECK (source_height IS NULL OR source_height > 0),
                    crop_left INTEGER NOT NULL CHECK (crop_left >= 0),
                    crop_top INTEGER NOT NULL CHECK (crop_top >= 0),
                    crop_right INTEGER NOT NULL CHECK (crop_right >= 0),
                    crop_bottom INTEGER NOT NULL CHECK (crop_bottom >= 0),
                    z_index INTEGER NOT NULL,
                    preserve_aspect_ratio INTEGER NOT NULL CHECK (preserve_aspect_ratio IN (0, 1)),
                    created_at_ms INTEGER NOT NULL CHECK (created_at_ms >= 0),
                    updated_at_ms INTEGER NOT NULL CHECK (updated_at_ms >= created_at_ms),
                    FOREIGN KEY (scene_id) REFERENCES recorder_scenes(id) ON DELETE CASCADE
                );
                "#,
            )
            .expect("create v4 recorder scene tables");
    }

    fn scene_source(source_id: &str, z_index: i64) -> SaveRecorderSceneSourceInput {
        SaveRecorderSceneSourceInput {
            crop_bottom: 0,
            crop_left: 0,
            crop_right: 0,
            crop_top: 0,
            display_name: source_id.to_string(),
            height: 1440,
            id: Uuid::now_v7().to_string(),
            preserve_aspect_ratio: true,
            source_id: source_id.to_string(),
            source_height: Some(1440),
            source_kind: "monitor".to_string(),
            source_width: Some(2560),
            width: 2560,
            x: 0,
            y: 0,
            z_index,
        }
    }
}
