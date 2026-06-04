use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
};

use rusqlite::Connection;
use rusqlite_migration::{Migrations, M};
use serde::Serialize;
use tauri::Manager;

const DATABASE_FILE_NAME: &str = "daydream.sqlite3";
const CORE_TABLES: [&str; 5] = [
    "recordings",
    "recording_tracks",
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

fn migrations() -> Migrations<'static> {
    Migrations::new(vec![M::up(
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
    )])
}

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;
    use uuid::Uuid;

    use super::{
        apply_migrations, collect_database_diagnostics_from_connection, enable_connection_pragmas,
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

        assert_eq!(schema_version, 1);
        assert_eq!(table_counts["recordings"], 0);
        assert_eq!(table_counts["recording_tracks"], 0);
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
}
