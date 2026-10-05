use super::super::super::*;

pub(crate) async fn reprocess_recording_ocr<R: tauri::Runtime>(
    app: &AppHandle<R>,
    storage: &AppStorage,
    recording_id: &str,
) -> Result<ProcessingJobRow, AiError> {
    let _lease = crate::recording_privacy::mutation_lease(recording_id).await;
    let settings = settings::load_app_settings(storage).map_err(settings_error)?;
    if !settings.ai.ocr.enabled {
        return Err(ai_error(
            "ai_ocr_disabled",
            "Enable OCR before reprocessing.",
        ));
    }
    let config = capture_ocr_job_config(&settings)?;
    let job=storage.with_connection_mut(|connection|{
        let transaction=connection.transaction()?;
        let recording=storage::get_recording(&transaction,recording_id)?;
        if recording.status!="completed" || !crate::recording_privacy::protection_ready(&transaction,recording_id)? {return Err(rusqlite::Error::InvalidQuery);}
        for job in storage::list_processing_jobs(&transaction,Some(recording_id))? {
            if matches!(job.status.as_str(),"queued"|"running"|"paused") && storage::list_processing_chunks(&transaction,&job.id)?.iter().any(|chunk|chunk.source_kind=="ocr"&&matches!(chunk.status.as_str(),"queued"|"running")) {return Err(rusqlite::Error::InvalidQuery);}
        }
        let segments=storage::list_recording_segments(&transaction,recording_id)?;
        let redactions=storage::list_redactions(&transaction,recording_id)?;
        let plans=visual_chunk_plans(&segments,&settings,&redactions,"ocr");
        if plans.is_empty(){return Err(rusqlite::Error::InvalidQuery);}
        let timestamp=now_ms();
        let job=storage::create_processing_job(&transaction,&CreateProcessingJobInput{created_at_ms:timestamp,error:None,id:Uuid::now_v7().to_string(),kind:"ocr".into(),recording_id:recording_id.into(),status:"queued".into(),updated_at_ms:timestamp})?;
        storage::save_processing_job_ocr_config(&transaction,&job.id,&config)?;
        for plan in plans {
            storage::create_processing_chunk(&transaction,&CreateProcessingChunkInput{attempt_count:0,created_at_ms:timestamp,end_offset_ms:Some(plan.end_offset_ms),error_code:None,id:Uuid::now_v7().to_string(),job_id:job.id.clone(),progress:0.0,recording_id:recording_id.into(),result_code:None,segment_id:Some(plan.segment_id),source_kind:"ocr".into(),start_offset_ms:Some(plan.start_offset_ms),status:"queued".into(),track_id:None,updated_at_ms:timestamp})?;
        }
        save_processing_notification(&transaction,ProcessingNotificationInput{actions:&processing_running_actions(&job.id),job_id:Some(&job.id),message:"OCR reprocessing is queued using the current OCR settings.",recording_id,status:"running",timestamp_ms:timestamp,title:"OCR reprocessing queued"})?;
        transaction.commit()?;Ok(job)
    }).map_err(|_|ai_error("ai_ocr_reprocess_unavailable","OCR reprocessing requires a completed, eligible recording with no active OCR work."))?;
    emit_processing_state(app, recording_id);
    Ok(job)
}
