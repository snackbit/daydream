use super::super::super::*;
use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::ai) struct UnitSelection {
    pub frame_timestamps: Option<Vec<i64>>,
    pub audio_ranges: Option<Vec<(i64, i64)>>,
}

pub(in crate::ai) fn load_unit_selection(
    storage: &AppStorage,
    chunk_id: &str,
) -> Result<UnitSelection, AiError> {
    match storage
        .with_connection(|connection| storage::processing_chunk_selection(connection, chunk_id))
        .map_err(storage_error)?
    {
        Some(json) => serde_json::from_str(&json).map_err(|_| {
            ai_error(
                "ai_processing_selection_invalid",
                "The saved retry selection is invalid.",
            )
        }),
        None => Ok(UnitSelection::default()),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RetryProcessingUnitsInput {
    pub job_id: String,
    pub chunk_id: Option<String>,
    pub unit_id: Option<String>,
    pub source_kind: Option<String>,
    #[serde(default)]
    pub use_current_settings: bool,
}

pub(crate) fn retry_processing_units_for_action(
    storage: &AppStorage,
    input: &RetryProcessingUnitsInput,
) -> Result<ProcessingJob, AiError> {
    if input
        .source_kind
        .as_deref()
        .is_some_and(|kind| !matches!(kind, "audio" | "ocr" | "vlm" | "activity"))
    {
        return Err(ai_error(
            "ai_processing_selection_invalid",
            "Choose a supported processing lane.",
        ));
    }
    let configuration = if input.use_current_settings {
        Some(
            serde_json::to_string(&settings::load_app_settings(storage).map_err(settings_error)?)
                .map_err(|_| {
                ai_error(
                    "ai_processing_config_invalid",
                    "Current settings could not be saved for retry.",
                )
            })?,
        )
    } else {
        None
    };
    let job=storage.with_connection_mut(|connection|{
        let transaction=connection.transaction()?;
        let job=storage::get_processing_job(&transaction,&input.job_id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)?;
        let chunks=storage::list_processing_chunks(&transaction,&input.job_id)?;
        let mut selected=0;
        for chunk in chunks {
            if input.chunk_id.as_ref().is_some_and(|id|id!=&chunk.id)||input.source_kind.as_ref().is_some_and(|kind|kind!=&chunk.source_kind)||!matches!(chunk.status.as_str(),"failed"|"cancelled"|"completed-with-errors") {continue;}
            let mut selection=UnitSelection::default();
            if matches!(chunk.source_kind.as_str(),"ocr"|"vlm") {
                let mut statement=transaction.prepare("SELECT id,frame_timestamp_ms FROM visual_frame_processing_states WHERE chunk_id=?1 AND status IN ('failed','cancelled','queued') ORDER BY frame_timestamp_ms")?;
                let frames=statement.query_map([&chunk.id],|row|Ok((row.get::<_,String>(0)?,row.get::<_,i64>(1)?)))?.collect::<rusqlite::Result<Vec<_>>>()?;
                let timestamps=frames.into_iter().filter(|(id,_)|input.unit_id.as_ref().is_none_or(|wanted|wanted==id||wanted==&chunk.id)).map(|(_,timestamp)|timestamp).collect::<Vec<_>>();
                if input.unit_id.as_ref().is_some_and(|id|id!=&chunk.id)&&timestamps.is_empty(){continue;}
                if !timestamps.is_empty(){selection.frame_timestamps=Some(timestamps);}
            } else if chunk.source_kind=="audio" {
                let mut statement=transaction.prepare("SELECT r.start_ms,r.end_ms FROM processing_audio_ranges r WHERE r.chunk_id=?1 AND r.status='failed' AND NOT EXISTS(SELECT 1 FROM processing_audio_ranges newer WHERE newer.chunk_id=r.chunk_id AND newer.start_ms=r.start_ms AND newer.end_ms=r.end_ms AND (newer.updated_at_ms>r.updated_at_ms OR (newer.updated_at_ms=r.updated_at_ms AND newer.rowid>r.rowid))) ORDER BY r.start_ms")?;
                let ranges=statement.query_map([&chunk.id],|row|Ok((row.get::<_,i64>(0)?,row.get::<_,i64>(1)?)))?.collect::<rusqlite::Result<Vec<_>>>()?;
                let ranges=ranges.into_iter().filter(|(start,end)|input.unit_id.as_ref().is_none_or(|wanted|wanted==&chunk.id||wanted==&format!("speech:{}:{start}:{end}",chunk.id))).collect::<Vec<_>>();
                if input.unit_id.as_ref().is_some_and(|id|id!=&chunk.id)&&ranges.is_empty(){continue;}
                if !ranges.is_empty(){selection.audio_ranges=Some(ranges);}
            } else if input.unit_id.as_ref().is_some_and(|id|id!=&chunk.id){continue;}
            let json=serde_json::to_string(&selection).map_err(|_|rusqlite::Error::InvalidQuery)?;
            transaction.execute("INSERT INTO processing_chunk_runtime(chunk_id,config_json,selection_json,phase) VALUES(?1,?2,?3,'queued') ON CONFLICT(chunk_id) DO UPDATE SET config_json=COALESCE(excluded.config_json,config_json),selection_json=excluded.selection_json,retry_count=0,next_retry_at_ms=NULL,phase='queued'",rusqlite::params![chunk.id,configuration,json])?;
            storage::update_processing_chunk_status(&transaction,&chunk.id,"queued",0.0,None,now_ms())?;
            selected+=1;
        }
        if selected==0 {return Err(rusqlite::Error::InvalidQuery);}
        let status=if job.status=="running"{"running"}else{"queued"};
        let job=storage::update_processing_job_status(&transaction,&job.id,status,None,now_ms())?;
        transaction.commit()?;Ok(job)
    }).map_err(|_|ai_error("ai_processing_retry_unavailable","Choose a failed section whose active attempt has stopped."))?;
    Ok(processing_job_from_row_with_storage(storage, job))
}
