use crate::storage::*;
use rusqlite::params;
use uuid::Uuid;

fn fixture(kind: &str) -> (AppStorage, ProcessingChunkRow) {
    let storage = test_storage(std::path::PathBuf::new());
    let chunk = storage
        .with_connection(|connection| {
            let recording = Uuid::now_v7().to_string();
            let job = Uuid::now_v7().to_string();
            create_recording(
                connection,
                &CreateRecordingInput {
                    id: recording.clone(),
                    active_scene_id: None,
                    created_at_ms: 1,
                    default_fps: 1.0,
                    notes: None,
                    output_root: None,
                    quality_label: "balanced-ocr".into(),
                    storage_dir: "/tmp/synthetic-retry-review".into(),
                },
            )?;
            create_processing_job(
                connection,
                &CreateProcessingJobInput {
                    id: job.clone(),
                    recording_id: recording.clone(),
                    kind: "ai-processing".into(),
                    status: "completed-with-errors".into(),
                    error: None,
                    created_at_ms: 1,
                    updated_at_ms: 1,
                },
            )?;
            let chunk = create_processing_chunk(
                connection,
                &CreateProcessingChunkInput {
                    id: Uuid::now_v7().to_string(),
                    job_id: job,
                    recording_id: recording,
                    source_kind: kind.into(),
                    segment_id: None,
                    track_id: None,
                    start_offset_ms: Some(0),
                    end_offset_ms: Some(3000),
                    status: "completed-with-errors".into(),
                    attempt_count: 1,
                    progress: 1.0,
                    error_code: Some("synthetic_provider_failure".into()),
                    result_code: None,
                    created_at_ms: 1,
                    updated_at_ms: 1,
                },
            )?;
            for (time, status) in [(0, "succeeded"), (1000, "failed"), (2000, "failed")] {
                upsert_visual_frame_processing_state(
                    connection,
                    &CreateVisualFrameProcessingStateInput {
                        id: Uuid::now_v7().to_string(),
                        recording_id: chunk.recording_id.clone(),
                        job_id: chunk.job_id.clone(),
                        chunk_id: chunk.id.clone(),
                        source_kind: kind.into(),
                        segment_id: None,
                        frame_timestamp_ms: time,
                        source_frame_ref: format!("synthetic:{time}"),
                        region_x: 0,
                        region_y: 0,
                        region_width: 4,
                        region_height: 2,
                        status: status.into(),
                        progress: 1.0,
                        error_code: if status == "failed" {
                            Some("synthetic_provider_failure".into())
                        } else {
                            None
                        },
                        output_count: if status == "succeeded" { 1 } else { 0 },
                        created_at_ms: 1,
                        updated_at_ms: 1,
                    },
                )?;
            }
            Ok(chunk)
        })
        .unwrap();
    (storage, chunk)
}
#[test]
fn review_whole_job_retry_preserves_successful_visual_output_and_checkpoints() {
    let (storage, chunk) = fixture("vlm");
    storage.with_connection(|connection|{
        for time in [0,2000]{
            create_vlm_observation(connection,&CreateVisualRegionInput{id:Uuid::now_v7().to_string(),recording_id:chunk.recording_id.clone(),job_id:chunk.job_id.clone(),chunk_id:chunk.id.clone(),frame_timestamp_ms:time,source_frame_ref:format!("synthetic:{time}"),region_x:0,region_y:0,region_width:4,region_height:2,text:"Synthetic observation".into(),confidence:None,provider:"openai-compatible".into(),model:Some("synthetic".into()),prompt_version:None,observation_kind:None})?;
        }
        retry_processing_job_chunks(connection,&chunk.job_id,2)?;
        let times=connection.prepare("SELECT frame_timestamp_ms FROM vlm_observations WHERE chunk_id=?1 ORDER BY frame_timestamp_ms")?.query_map([&chunk.id],|row|row.get::<_,i64>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
        assert_eq!(times,vec![0],"Committed observations must survive; incomplete output must not");
        let states=list_visual_frame_processing_states(connection,&chunk.recording_id)?;
        assert_eq!(states.iter().filter(|state|state.status=="succeeded").count(),1);
        assert_eq!(states.iter().filter(|state|state.status=="failed").count(),2,"Unrecovered sections must remain visible");
        Ok(())
    }).unwrap();
}
#[test]
fn review_whole_job_retry_clears_targeted_selection_and_keeps_unrecovered_frames() {
    let (storage, chunk) = fixture("ocr");
    storage.with_connection(|connection|{
        connection.execute("INSERT INTO processing_chunk_runtime(chunk_id,selection_json) VALUES(?1,?2)",params![chunk.id,r#"{"frameTimestamps":[1000]}"#])?;
        // A targeted retry has completed A; B still needs recovery.
        connection.execute("UPDATE visual_frame_processing_states SET status='succeeded',error_code=NULL WHERE chunk_id=?1 AND frame_timestamp_ms=1000",[&chunk.id])?;
        retry_processing_job_chunks(connection,&chunk.job_id,2)?;
        assert_eq!(processing_chunk_selection(connection,&chunk.id)?,None,"A whole-job retry must cover all remaining units");
        assert_eq!(first_processing_chunk_failure(connection,&chunk.id)?.as_deref(),Some("synthetic_provider_failure"));
        assert_eq!(list_visual_frame_processing_states(connection,&chunk.recording_id)?.iter().filter(|state|state.status=="succeeded").count(),2);
        Ok(())
    }).unwrap();
}
