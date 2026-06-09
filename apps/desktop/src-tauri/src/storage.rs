use std::{
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

use rusqlite::Connection;
use tauri::Manager;

mod ai;
mod diagnostics;
mod portal_tokens;
mod recordings;
mod scenes;
mod schema;
mod segments;
mod settings;

pub use ai::{
    create_activity_interval, create_frame_similarity_entry, create_ocr_region,
    create_processing_chunk, create_transcript_segment, create_transcript_word,
    create_vlm_observation, save_ai_model_asset, AiModelAssetRow, CreateActivityIntervalInput,
    CreateFrameSimilarityEntryInput, CreateProcessingChunkInput, CreateTranscriptSegmentInput,
    CreateTranscriptWordInput, CreateVisualRegionInput, ProcessingChunkRow, SaveAiModelAssetInput,
    TranscriptSegmentRow,
};
pub use diagnostics::{collect_database_diagnostics, DatabaseDiagnostics};
pub use portal_tokens::{
    delete_portal_restore_token, get_portal_restore_token, mark_portal_restore_token_used,
    upsert_portal_restore_token, PortalRestoreTokenRow, LINUX_SCREENCAST_MONITOR_RESTORE_TOKEN_ID,
};
pub use recordings::{
    create_recording, delete_recording, get_latest_recording_id, get_recording,
    is_active_recording_status, is_deletable_recording_status, list_recordings,
    mark_recording_completed, mark_recording_started, recover_interrupted_recordings,
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
pub use settings::{
    get_app_setting, list_app_settings, reset_app_settings, save_app_setting, save_app_settings,
    AppSettingRow, SaveAppSettingInput,
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
    recordings::recover_interrupted_recordings(&connection, now_ms())?;

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

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests;
