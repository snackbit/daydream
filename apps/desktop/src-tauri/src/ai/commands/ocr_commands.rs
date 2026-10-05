use crate::ai::{GetOcrTimelineFramesInput, OcrTimelineFramePage};
use crate::ai::{ProcessingJob, ReprocessRecordingOcrInput};
use crate::{
    ai::{AiError, GetOcrDocumentInput, GetOcrHistoryInput, OcrDocumentSelection, OcrHistoryPage},
    storage::{self, AppStorage},
};
use tauri::State;

fn invalid_input() -> AiError {
    AiError {
        code: "ai_ocr_query_invalid".into(),
        message: "OCR query requires a recording, source, and valid timeline position.".into(),
    }
}
fn storage_error(_: rusqlite::Error) -> AiError {
    AiError {
        code: "ai_ocr_query_failed".into(),
        message: "OCR documents could not be loaded.".into(),
    }
}

#[tauri::command(rename = "getOcrTimelineFrames")]
pub fn get_ocr_timeline_frames(
    input: GetOcrTimelineFramesInput,
    storage: State<'_, AppStorage>,
) -> Result<OcrTimelineFramePage, AiError> {
    if input.recording_id.is_empty()
        || input.start_timestamp_ms.is_some_and(|value| value < 0)
        || input.end_timestamp_ms.is_some_and(|value| value < 0)
        || matches!((input.start_timestamp_ms,input.end_timestamp_ms),(Some(start),Some(end)) if end<=start)
        || input
            .cursor
            .as_ref()
            .is_some_and(|cursor| cursor.timestamp_ms < 0 || cursor.id.is_empty())
    {
        return Err(invalid_input());
    }
    storage
        .with_connection(|connection| storage::get_ocr_timeline_frames(connection, &input))
        .map_err(storage_error)
}

#[tauri::command(rename = "getOcrDocument")]
pub fn get_ocr_document(
    input: GetOcrDocumentInput,
    storage: State<'_, AppStorage>,
) -> Result<OcrDocumentSelection, AiError> {
    if input.recording_id.is_empty() || input.source_track_id.is_empty() || input.timestamp_ms < 0 {
        return Err(invalid_input());
    }
    storage
        .with_connection(|connection| storage::get_ocr_document(connection, &input))
        .map_err(storage_error)
}
#[tauri::command(rename = "getOcrHistory")]
pub fn get_ocr_history(
    input: GetOcrHistoryInput,
    storage: State<'_, AppStorage>,
) -> Result<OcrHistoryPage, AiError> {
    if input.recording_id.is_empty()
        || input
            .cursor
            .as_ref()
            .is_some_and(|cursor| cursor.timestamp_ms < 0 || cursor.id.is_empty())
    {
        return Err(invalid_input());
    }
    storage
        .with_connection(|connection| storage::get_ocr_history(connection, &input))
        .map_err(storage_error)
}

#[tauri::command(rename = "reprocessRecordingOcr")]
pub async fn reprocess_recording_ocr(
    input: ReprocessRecordingOcrInput,
    app: tauri::AppHandle,
    storage: State<'_, AppStorage>,
) -> Result<ProcessingJob, AiError> {
    if input.recording_id.is_empty() {
        return Err(invalid_input());
    }
    let job = crate::ai::reprocess_recording_ocr(&app, &storage, &input.recording_id).await?;
    Ok(super::super::results::processing_job_from_row_with_storage(
        &storage, job,
    ))
}
