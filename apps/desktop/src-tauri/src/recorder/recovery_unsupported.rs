//! Preserve the IPC command on platforms without the media recovery worker.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RecoverLibraryRecordingInput {
    recording_id: String,
}

#[tauri::command(rename = "recoverLibraryRecording")]
pub(crate) async fn recover_library_recording(
    input: RecoverLibraryRecordingInput,
) -> Result<(), String> {
    let _recording_id = input.recording_id;
    Err("Recording recovery is not available on this platform.".into())
}
