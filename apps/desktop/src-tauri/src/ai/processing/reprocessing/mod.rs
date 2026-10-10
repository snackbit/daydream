//! Admission and preview share a deterministic plan, bound to current settings,
//! media provenance, successful results and privacy state.
mod planning;
mod types;
mod utterances;
use super::super::*;
pub(crate) use types::*;

pub(crate) fn whole_recording(
    storage: &AppStorage,
    recording_id: &str,
    operation: ReprocessingOperation,
) -> Result<ProcessingJob, AiError> {
    let targets = storage
        .with_connection(|connection| {
            let mut ids = BTreeSet::new();
            for segment in storage::list_recording_segments(connection, recording_id)? {
                if operation == ReprocessingOperation::Transcription {
                    for role in ["microphone", "desktop-audio"] {
                        if let Some(id) = crate::recording_privacy::source_track_id(&segment, role)
                        {
                            ids.insert(id);
                        }
                    }
                } else {
                    for stream in storage::segment_video_streams(connection, &segment)? {
                        ids.insert(stream.track_id);
                    }
                }
            }
            Ok(ids
                .into_iter()
                .map(|track_id| ReprocessingTarget {
                    track_id,
                    ranges: None,
                })
                .collect())
        })
        .map_err(storage_error)?;
    let input = PreviewRecordingReprocessingInput {
        recording_id: recording_id.into(),
        targets,
        operations: Some(vec![operation]),
    };
    let preview = preview(storage, &input)?;
    let kind = if input
        .operations
        .as_ref()
        .is_some_and(|o| o == &vec![ReprocessingOperation::Ocr])
    {
        "ocr"
    } else {
        AI_PROCESSING_JOB_KIND
    };
    start_with_kind(
        storage,
        StartRecordingReprocessingInput {
            recording_id: input.recording_id,
            targets: input.targets,
            operations: input.operations,
            plan_fingerprint: preview.plan_fingerprint,
        },
        kind,
    )
}

pub(crate) fn preview(
    storage: &AppStorage,
    input: &PreviewRecordingReprocessingInput,
) -> Result<RecordingReprocessingPreview, AiError> {
    let settings = settings::load_app_settings(storage).map_err(settings_error)?;
    storage
        .with_connection(|connection| {
            planning::plan(connection, &settings, input).map(|p| p.preview)
        })
        .map_err(|_| {
            ai_error(
                "ai_reprocessing_invalid",
                "Choose eligible audio or screen tracks and valid time ranges.",
            )
        })
}

pub(crate) fn progress(
    connection: &rusqlite::Connection,
    recording_id: &str,
) -> rusqlite::Result<(Vec<RecordingReprocessingProgress>, String)> {
    let mut statement=connection.prepare("SELECT rg.id,rg.source_track_id,rg.source_role,rg.source_kind,rg.start_ms,rg.end_ms,rg.job_id,CASE WHEN rg.committed=1 THEN 'succeeded' ELSE j.status END,rg.committed FROM processing_replacement_groups rg JOIN processing_jobs j ON j.id=rg.job_id WHERE rg.recording_id=?1 ORDER BY j.created_at_ms,rg.start_ms,rg.id")?;
    let scopes = statement
        .query_map([recording_id], |r| {
            let kind: String = r.get(3)?;
            let role: Option<String> = r.get(2)?;
            let operation = match kind.as_str() {
                "audio" => ReprocessingOperation::Transcription,
                "ocr" => ReprocessingOperation::Ocr,
                "vlm" => ReprocessingOperation::ScreenObservations,
                _ => return Err(rusqlite::Error::InvalidQuery),
            };
            Ok(RecordingReprocessingProgress {
                scope: RecordingReprocessingScope {
                    id: r.get(0)?,
                    track_id: r.get(1)?,
                    source_label: if role.as_deref() == Some("microphone") {
                        "Microphone"
                    } else if kind == "audio" {
                        "Desktop audio"
                    } else {
                        "Screen"
                    }
                    .into(),
                    operation,
                    start_offset_ms: r.get(4)?,
                    end_offset_ms: r.get(5)?,
                },
                job_id: r.get(6)?,
                status: r.get(7)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let frames:i64=connection.query_row("SELECT COUNT(*) FROM processing_replacement_frames rf JOIN processing_chunks p ON p.id=rf.chunk_id WHERE p.recording_id=?1",[recording_id],|r|r.get(0))?;
    let revision = format!(
        "{:x}",
        Sha1::digest(
            serde_json::to_vec(&(&scopes, frames)).map_err(|_| rusqlite::Error::InvalidQuery)?
        )
    );
    Ok((scopes, revision))
}

pub(crate) fn start(
    storage: &AppStorage,
    input: StartRecordingReprocessingInput,
) -> Result<ProcessingJob, AiError> {
    start_with_kind(storage, input, AI_PROCESSING_JOB_KIND)
}
fn start_with_kind(
    storage: &AppStorage,
    input: StartRecordingReprocessingInput,
    kind: &str,
) -> Result<ProcessingJob, AiError> {
    let settings = settings::load_app_settings(storage).map_err(settings_error)?;
    let configuration = serde_json::to_string(&settings).map_err(|_| {
        ai_error(
            "ai_reprocessing_config_invalid",
            "Model settings could not be saved.",
        )
    })?;
    let expected = input.plan_fingerprint;
    let input = PreviewRecordingReprocessingInput {
        recording_id: input.recording_id,
        targets: input.targets,
        operations: input.operations,
    };
    let job=storage.with_connection_mut(|connection|{
        let transaction=connection.transaction()?;
        let plan=planning::plan(&transaction,&settings,&input)?;
        if plan.preview.blocked_reason.is_some()||plan.preview.plan_fingerprint!=expected {return Err(rusqlite::Error::InvalidQuery)}
        let timestamp=now_ms();
        let job=storage::create_processing_job(&transaction,&CreateProcessingJobInput{id:Uuid::now_v7().to_string(),recording_id:input.recording_id.clone(),kind:kind.into(),status:"queued".into(),created_at_ms:timestamp,updated_at_ms:timestamp,error:None})?;
        storage::set_processing_job_runtime(&transaction,&job.id,false,&configuration)?;
        storage::save_processing_job_ocr_config(&transaction,&job.id,&capture_ocr_job_config(&settings).map_err(|_|rusqlite::Error::InvalidQuery)?)?;
        for scope in plan.preview.scopes {
            let group=Uuid::now_v7().to_string();
            let children=plan.sections.iter().filter(|s|s.source_track_id==scope.track_id&&s.operation==scope.operation&&s.start>=scope.start_offset_ms&&s.end<=scope.end_offset_ms);
            let role=if scope.operation==ReprocessingOperation::Transcription {plan.sections.iter().find(|s|s.source_track_id==scope.track_id).and_then(|s|s.role.clone())}else{None};
            transaction.execute("INSERT INTO processing_replacement_groups(id,recording_id,job_id,source_track_id,source_role,source_kind,start_ms,end_ms) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",rusqlite::params![group,input.recording_id,job.id,scope.track_id,role,scope.operation.kind(),scope.start_offset_ms,scope.end_offset_ms])?;
            for section in children {
                let track=if let Some(role)=&section.role {Some(ensure_audio_track(&transaction,&input.recording_id,role,&section.label,timestamp)?.id)}else{section.native_track_id.clone()};
                let chunk=storage::create_processing_chunk(&transaction,&CreateProcessingChunkInput{id:Uuid::now_v7().to_string(),job_id:job.id.clone(),recording_id:input.recording_id.clone(),source_kind:section.operation.kind().into(),segment_id:Some(section.segment_id.clone()),track_id:track,start_offset_ms:Some(section.start),end_offset_ms:Some(section.end),attempt_count:0,created_at_ms:timestamp,updated_at_ms:timestamp,status:"queued".into(),progress:0.0,error_code:None,result_code:None})?;
                transaction.execute("INSERT INTO processing_chunk_runtime(chunk_id,config_json,phase) VALUES(?1,?2,'queued')",rusqlite::params![chunk.id,configuration])?;
                transaction.execute("INSERT INTO processing_replacement_chunks(chunk_id,group_id) VALUES(?1,?2)",rusqlite::params![chunk.id,group])?;
            }
        }
        save_processing_notification(&transaction,ProcessingNotificationInput{actions:&processing_running_actions(&job.id),job_id:Some(&job.id),message:"Reprocessing is queued using the current model settings.",recording_id:&input.recording_id,status:"running",timestamp_ms:timestamp,title:"Reprocessing queued"})?;
        transaction.commit()?;Ok(job)
    }).map_err(|_|ai_error("ai_reprocessing_preview_changed","The recording or settings changed. Refresh the preview before starting."))?;
    Ok(processing_job_from_row_with_storage(storage, job))
}
