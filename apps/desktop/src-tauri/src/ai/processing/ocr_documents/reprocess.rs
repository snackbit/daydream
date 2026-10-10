use super::super::super::*;

pub(crate) async fn reprocess_recording_ocr<R: tauri::Runtime>(
    app: &AppHandle<R>,
    storage: &AppStorage,
    recording_id: &str,
) -> Result<ProcessingJobRow, AiError> {
    let _lease = crate::recording_privacy::mutation_lease(recording_id).await;
    let job = processing::reprocessing::whole_recording(
        storage,
        recording_id,
        processing::reprocessing::ReprocessingOperation::Ocr,
    )?;
    emit_processing_state(app, recording_id);
    storage
        .with_connection(|connection| {
            storage::get_processing_job(connection, &job.id)?
                .ok_or(rusqlite::Error::QueryReturnedNoRows)
        })
        .map_err(storage_error)
}
