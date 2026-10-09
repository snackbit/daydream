use super::super::super::*;

pub(crate) fn reprocess_recording_audio(
    storage: &AppStorage,
    recording_id: &str,
) -> Result<ProcessingJob, AiError> {
    let settings = settings::load_app_settings(storage).map_err(settings_error)?;
    let configuration = serde_json::to_string(&settings).map_err(|_| {
        ai_error(
            "ai_processing_config_invalid",
            "Current transcription settings could not be saved.",
        )
    })?;
    let job=storage.with_connection_mut(|connection|{
        let transaction=connection.transaction()?;
        let recording=storage::get_recording(&transaction,recording_id)?;
        if !matches!(recording.status.as_str(),"completed"|"failed"|"recovered-partial")||!crate::recording_privacy::protection_ready(&transaction,recording_id)?{return Err(rusqlite::Error::InvalidQuery);}
        let existing=storage::get_latest_processing_job_for_recording(&transaction,recording_id,AI_PROCESSING_JOB_KIND)?;
        if existing.as_ref().is_some_and(|job|matches!(job.status.as_str(),"running"|"queued"|"paused")){return Err(rusqlite::Error::InvalidQuery);}
        let segments=storage::list_recording_segments(&transaction,recording_id)?;
        let cuts=storage::list_redactions(&transaction,recording_id)?;
        let plans=audio_chunk_plans(&segments,&cuts);
        if plans.is_empty(){return Err(rusqlite::Error::InvalidQuery);}
        let timestamp=now_ms();
        let job=if let Some(job)=existing {job}else{
            let job=storage::create_processing_job(&transaction,&CreateProcessingJobInput{id:Uuid::now_v7().to_string(),recording_id:recording_id.into(),kind:AI_PROCESSING_JOB_KIND.into(),status:"queued".into(),created_at_ms:timestamp,updated_at_ms:timestamp,error:None})?;
            storage::save_processing_job_ocr_config(&transaction,&job.id,&capture_ocr_job_config(&settings).map_err(|_|rusqlite::Error::InvalidQuery)?)?;
            storage::set_processing_job_runtime(&transaction,&job.id,false,&configuration)?;job
        };
        // An explicit source reprocess replaces transcripts/checkpoints while
        // leaving OCR, screen observations, activity, and saved media intact.
        transaction.execute("DELETE FROM processing_chunks WHERE job_id=?1 AND source_kind='audio'",[&job.id])?;
        for plan in plans {
            let label=if plan.source_role=="microphone"{"Microphone"}else{"Desktop audio"};
            let track=ensure_audio_track(&transaction,recording_id,&plan.source_role,label,timestamp)?;
            let chunk=storage::create_processing_chunk(&transaction,&CreateProcessingChunkInput{id:Uuid::now_v7().to_string(),job_id:job.id.clone(),recording_id:recording_id.into(),source_kind:"audio".into(),segment_id:Some(plan.segment_id),track_id:Some(track.id),start_offset_ms:Some(plan.start_offset_ms),end_offset_ms:Some(plan.end_offset_ms),attempt_count:0,created_at_ms:timestamp,updated_at_ms:timestamp,status:"queued".into(),progress:0.0,error_code:None,result_code:None})?;
            transaction.execute("INSERT INTO processing_chunk_runtime(chunk_id,config_json,phase) VALUES(?1,?2,'queued')",rusqlite::params![chunk.id,configuration])?;
        }
        let job=storage::update_processing_job_status(&transaction,&job.id,"queued",None,timestamp)?;
        transaction.commit()?;Ok(job)
    }).map_err(|_|ai_error("ai_processing_reprocess_unavailable","Stop active processing first. Audio reprocessing requires finalized, eligible audio sections."))?;
    Ok(processing_job_from_row_with_storage(storage, job))
}
