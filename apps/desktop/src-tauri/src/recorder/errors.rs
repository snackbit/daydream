use rusqlite::Error as SqliteError;

use super::types::{RecorderError, RecorderErrorCategory};

const RECORDER_BACKEND_NOT_IMPLEMENTED: &str = "recorder_backend_not_implemented";

pub(crate) fn unavailable_error() -> RecorderError {
    RecorderError {
        category: RecorderErrorCategory::Unavailable,
        code: RECORDER_BACKEND_NOT_IMPLEMENTED.to_string(),
        details: None,
        message: "Recorder backend is not implemented yet.".to_string(),
    }
}

pub(crate) fn lifecycle_error(code: &str, message: &str) -> RecorderError {
    RecorderError {
        category: RecorderErrorCategory::Lifecycle,
        code: code.to_string(),
        details: None,
        message: message.to_string(),
    }
}

pub(crate) fn filesystem_error(error: std::io::Error) -> RecorderError {
    RecorderError {
        category: RecorderErrorCategory::Filesystem,
        code: "recorder_storage_unavailable".to_string(),
        details: None,
        message: format!("Recorder storage is unavailable: {error}"),
    }
}

pub(crate) fn storage_error(error: SqliteError) -> RecorderError {
    RecorderError {
        category: RecorderErrorCategory::Dependency,
        code: "recorder_storage_error".to_string(),
        details: None,
        message: format!("Recorder storage failed: {error}"),
    }
}

pub(crate) fn storage_error_message(message: &str) -> RecorderError {
    RecorderError {
        category: RecorderErrorCategory::Dependency,
        code: "recorder_storage_error".to_string(),
        details: None,
        message: message.to_string(),
    }
}

pub(crate) fn invalid_scene_error(field: &str) -> RecorderError {
    RecorderError {
        category: RecorderErrorCategory::Lifecycle,
        code: "recorder_invalid_scene".to_string(),
        details: None,
        message: format!("Recorder scene field {field} is invalid."),
    }
}

pub(crate) fn scene_or_storage_error(error: SqliteError, scene_id: &str) -> RecorderError {
    match error {
        SqliteError::QueryReturnedNoRows => lifecycle_error(
            "recorder_scene_not_found",
            &format!("Recorder scene {scene_id} was not found."),
        ),
        other => storage_error(other),
    }
}
