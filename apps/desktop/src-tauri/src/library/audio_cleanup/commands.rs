use super::*;
use crate::storage::AppStorage;
use tauri::{Manager, State};

#[tauri::command(rename = "getAudioCleanup")]
pub fn get_audio_cleanup(
    recording_id: String,
    storage: State<'_, AppStorage>,
) -> Result<AudioCleanupState, String> {
    read_state(storage.inner(), &recording_id)
}
#[tauri::command(rename = "applyAudioCleanup")]
pub async fn apply_audio_cleanup(
    app: tauri::AppHandle,
    input: ApplyAudioCleanupInput,
) -> Result<AudioCleanupState, String> {
    service::apply(app, input).await
}
#[tauri::command(rename = "cancelAudioCleanup")]
pub async fn cancel_audio_cleanup(
    app: tauri::AppHandle,
    recording_id: String,
) -> Result<AudioCleanupState, String> {
    let storage = app.state::<AppStorage>();
    cancel_recording(&storage, &recording_id).await?;
    read_state(&storage, &recording_id)
}
#[tauri::command(rename = "previewAudioCleanup")]
pub async fn preview_audio_cleanup(
    app: tauri::AppHandle,
    input: PreviewAudioCleanupInput,
) -> Result<AudioCleanupPreview, String> {
    preview::preview(app, input).await
}
