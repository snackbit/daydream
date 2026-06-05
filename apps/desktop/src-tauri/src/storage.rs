use std::{
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
};

use rusqlite::Connection;
use tauri::Manager;

mod diagnostics;
mod portal_tokens;
mod recordings;
mod scenes;
mod schema;
mod segments;

pub use diagnostics::{collect_database_diagnostics, DatabaseDiagnostics};
pub use portal_tokens::{
    get_portal_restore_token, mark_portal_restore_token_used, upsert_portal_restore_token,
    PortalRestoreTokenRow,
};
pub use recordings::{
    create_recording, get_recording, mark_recording_completed, mark_recording_started,
    update_recording_status, CreateRecordingInput, RecordingRow,
};
pub use scenes::{
    get_latest_recorder_scene, get_recorder_scene, save_recorder_scene, RecorderSceneRow,
    RecorderSceneSourceRow, SaveRecorderSceneInput, SaveRecorderSceneSourceInput,
};
pub use segments::{
    create_recording_segment, fail_recording_segment, finalize_recording_segment,
    list_recording_segments, CreateRecordingSegmentInput, FailRecordingSegmentInput,
    FinalizeRecordingSegmentInput, RecordingSegmentRow,
};

#[cfg(test)]
use diagnostics::collect_database_diagnostics_from_connection;
#[cfg(test)]
use schema::{apply_migrations, enable_connection_pragmas, migrations};

const DATABASE_FILE_NAME: &str = "daydream.sqlite3";

pub struct AppStorage {
    connection: Mutex<Connection>,
    path: PathBuf,
}

pub fn initialize(app: &tauri::AppHandle) -> Result<AppStorage, Box<dyn std::error::Error>> {
    let app_data_dir = app.path().app_data_dir()?;
    fs::create_dir_all(&app_data_dir)?;

    let db_path = app_data_dir.join(DATABASE_FILE_NAME);
    let mut connection = schema::open_connection(&db_path)?;
    schema::apply_migrations(&mut connection)?;

    Ok(AppStorage {
        connection: Mutex::new(connection),
        path: db_path,
    })
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
    schema::enable_connection_pragmas(&connection).expect("enable sqlite pragmas");
    schema::apply_migrations(&mut connection).expect("apply migrations");

    AppStorage {
        connection: Mutex::new(connection),
        path,
    }
}

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

#[cfg(test)]
mod tests;
