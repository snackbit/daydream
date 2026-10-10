use super::*;
use processing::reprocessing::*;
mod regressions;

struct Fixture {
    storage: AppStorage,
    chunk: ProcessingChunkRow,
    track: RecordingTrackRow,
    root: PathBuf,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
fn fixture() -> Fixture {
    let root = std::env::temp_dir().join(format!("daydream-replacement-{}", Uuid::now_v7()));
    fs::create_dir_all(&root).unwrap();
    let file = root.join("synthetic.mkv");
    fs::write(&file, b"synthetic media metadata fixture").unwrap();
    let storage = storage::test_storage(PathBuf::new());
    let (chunk,track)=storage.with_connection(|connection|{
        let (chunk,track)=test_transcription_chunk(connection);
        storage::mark_recording_started(connection,&chunk.recording_id,1)?;
        connection.execute("UPDATE recording_segments SET file_path=?2 WHERE id=?1",rusqlite::params![chunk.segment_id,file.to_string_lossy()])?;
        storage::finalize_recording_segment(connection,&storage::FinalizeRecordingSegmentInput{id:chunk.segment_id.clone().unwrap(),duration_ms:10000,ended_at_ms:10001,updated_at_ms:10001,video_stream_index:Some(0),desktop_audio_stream_index:Some(1),microphone_audio_stream_index:Some(2),probe_json:None})?;
        connection.execute("UPDATE recording_segments SET timeline_base_ms=0,timeline_basis_version=1 WHERE id=?1",[&chunk.segment_id])?;
        storage::mark_recording_completed(connection,&chunk.recording_id,10001)?;
        storage::update_processing_chunk_status(connection,&chunk.id,"succeeded",1.0,None,10001)?;
        storage::update_processing_job_status(connection,&chunk.job_id,"succeeded",None,10001)?;
        speech(connection,&chunk,&track,"old",1000,3000)?;
        Ok((chunk,track))
    }).unwrap();
    Fixture {
        storage,
        chunk,
        track,
        root,
    }
}
fn speech(
    connection: &rusqlite::Connection,
    chunk: &ProcessingChunkRow,
    track: &RecordingTrackRow,
    text: &str,
    start: i64,
    end: i64,
) -> rusqlite::Result<()> {
    storage::create_transcript_segment(
        connection,
        &CreateTranscriptSegmentInput {
            id: Uuid::now_v7().to_string(),
            recording_id: chunk.recording_id.clone(),
            segment_id: chunk.segment_id.clone().unwrap(),
            track_id: track.id.clone(),
            source_role: "microphone".into(),
            start_offset_ms: start,
            end_offset_ms: end,
            provider: "synthetic".into(),
            model: None,
            text: text.into(),
            confidence: None,
            job_id: chunk.job_id.clone(),
            chunk_id: chunk.id.clone(),
            created_at_ms: 1,
        },
    )?;
    Ok(())
}
fn request(f: &Fixture) -> PreviewRecordingReprocessingInput {
    PreviewRecordingReprocessingInput {
        recording_id: f.chunk.recording_id.clone(),
        targets: vec![ReprocessingTarget {
            track_id: "microphone-2".into(),
            ranges: Some(vec![ReprocessingRange {
                start_offset_ms: 1500,
                end_offset_ms: 2000,
            }]),
        }],
        operations: Some(vec![ReprocessingOperation::Transcription]),
    }
}
fn start_preview(f: &Fixture, input: PreviewRecordingReprocessingInput) -> ProcessingJob {
    let plan = preview(&f.storage, &input).unwrap();
    start(
        &f.storage,
        StartRecordingReprocessingInput {
            recording_id: input.recording_id,
            targets: input.targets,
            operations: input.operations,
            plan_fingerprint: plan.plan_fingerprint,
        },
    )
    .unwrap()
}

#[test]
fn scoped_reprocessing_expands_utterances_pins_settings_and_retains_failed_output() {
    let f = fixture();
    let input = request(&f);
    let plan = preview(&f.storage, &input).unwrap();
    assert_eq!(plan.scopes.len(), 1);
    assert_eq!(
        (plan.scopes[0].start_offset_ms, plan.scopes[0].end_offset_ms),
        (1000, 3000)
    );
    let job = start_preview(&f, input);
    f.storage.with_connection_mut(|connection|{
        let transaction=connection.transaction()?;
        let child=storage::list_processing_chunks(&transaction,&job.id)?.remove(0);
        assert_eq!(child.track_id.as_deref(),Some(f.track.id.as_str()));assert!(storage::processing_chunk_config(&transaction,&child.id)?.is_some());
        speech(&transaction,&child,&f.track,"candidate",1000,3000)?;
        storage::update_processing_chunk_status(&transaction,&child.id,"running",0.5,None,child.created_at_ms+1)?;
        storage::update_processing_job_status(&transaction,&child.job_id,"running",None,child.created_at_ms+1)?;
        storage::refresh_evidence(&transaction)?;
        let recall:Vec<String>=transaction.prepare("SELECT text FROM recall_evidence WHERE recording_id=?1")?.query_map([&child.recording_id],|r|r.get(0))?.collect::<rusqlite::Result<_>>()?;
        assert_eq!(recall,vec!["old"],"Staged results must stay out of Recall; pending work must not hide the prior result");
        storage::update_processing_chunk_status(&transaction,&child.id,"failed",1.0,Some("synthetic_failure"),child.created_at_ms+2)?;
        storage::publish_ready_replacement(&transaction,&child.id)?;
        assert_eq!(storage::list_transcript_segments(&transaction,&child.recording_id)?.iter().map(|s|s.text.as_str()).collect::<Vec<_>>(),vec!["old"]);
        storage::update_processing_chunk_status(&transaction,&child.id,"succeeded",1.0,None,child.created_at_ms+3)?;
        storage::publish_ready_replacement(&transaction,&child.id)?;
        assert_eq!(storage::list_transcript_segments(&transaction,&child.recording_id)?.iter().map(|s|s.text.as_str()).collect::<Vec<_>>(),vec!["candidate"]);
        storage::refresh_evidence(&transaction)?;
        let recall:Vec<String>=transaction.prepare("SELECT text FROM recall_evidence WHERE recording_id=?1")?.query_map([&child.recording_id],|r|r.get(0))?.collect::<rusqlite::Result<_>>()?;
        assert_eq!(recall,vec!["candidate"]);
        transaction.commit()?;Ok(())
    }).unwrap();
}

#[test]
fn stale_or_redacted_reprocessing_preview_is_rejected_without_admitting_a_job() {
    let f = fixture();
    let input = request(&f);
    let plan = preview(&f.storage, &input).unwrap();
    f.storage
        .with_connection(|c| {
            storage::create_redaction(
                c,
                &storage::CreateRedactionInput {
                    id: Uuid::now_v7().to_string(),
                    recording_id: f.chunk.recording_id.clone(),
                    start_offset_ms: 1700,
                    end_offset_ms: 1800,
                    track_ids_json: "[\"microphone-2\"]".into(),
                    reason: None,
                    created_at_ms: 2,
                },
            )?;
            Ok(())
        })
        .unwrap();
    assert!(start(
        &f.storage,
        StartRecordingReprocessingInput {
            recording_id: input.recording_id,
            targets: input.targets,
            operations: input.operations,
            plan_fingerprint: plan.plan_fingerprint
        }
    )
    .is_err());
    f.storage
        .with_connection(|c| {
            assert_eq!(
                storage::list_processing_jobs(c, Some(&f.chunk.recording_id))?.len(),
                1
            );
            assert_eq!(
                storage::list_transcript_segments(c, &f.chunk.recording_id)?.len(),
                1
            );
            Ok(())
        })
        .unwrap();
}

#[test]
fn reprocessing_preview_respects_durable_speech_units_and_source_specific_redactions() {
    let f = fixture();
    f.storage
        .with_connection(|c| {
            storage::save_audio_range_checkpoint(c, &f.chunk.id, 500, 3500, "synthetic-vad", None)?;
            storage::create_redaction(
                c,
                &storage::CreateRedactionInput {
                    id: Uuid::now_v7().to_string(),
                    recording_id: f.chunk.recording_id.clone(),
                    start_offset_ms: 0,
                    end_offset_ms: 10000,
                    track_ids_json: "[\"desktop-audio-1\"]".into(),
                    reason: None,
                    created_at_ms: 2,
                },
            )?;
            Ok(())
        })
        .unwrap();
    let plan = preview(&f.storage, &request(&f)).unwrap();
    assert_eq!(plan.scopes.len(), 1);
    assert_eq!(
        (plan.scopes[0].start_offset_ms, plan.scopes[0].end_offset_ms),
        (500, 3500)
    );
    assert_eq!(plan.scopes[0].track_id, "microphone-2");
}

#[test]
fn reprocessing_cross_chunk_utterance_publishes_only_after_every_child_succeeds() {
    let f = fixture();
    f.storage
        .with_connection(|c| {
            c.execute(
                "UPDATE recording_segments SET duration_ms=40000,ended_at_ms=40001 WHERE id=?1",
                [&f.chunk.segment_id],
            )?;
            speech(c, &f.chunk, &f.track, "spanning utterance", 29000, 31000)?;
            Ok(())
        })
        .unwrap();
    let mut input = request(&f);
    input.targets[0].ranges = Some(vec![ReprocessingRange {
        start_offset_ms: 29500,
        end_offset_ms: 30500,
    }]);
    let job = start_preview(&f, input);
    f.storage
        .with_connection_mut(|c| {
            let tx = c.transaction()?;
            let children = storage::list_processing_chunks(&tx, &job.id)?;
            assert_eq!(children.len(), 2);
            for (i, child) in children.iter().enumerate() {
                speech(
                    &tx,
                    child,
                    &f.track,
                    &format!("replacement {i}"),
                    child.start_offset_ms.unwrap(),
                    child.end_offset_ms.unwrap(),
                )?;
            }
            let timestamp = children[0].created_at_ms + 1;
            storage::update_processing_chunk_status(
                &tx,
                &children[0].id,
                "succeeded",
                1.0,
                None,
                timestamp,
            )?;
            storage::publish_ready_replacement(&tx, &children[0].id)?;
            storage::update_processing_chunk_status(
                &tx,
                &children[1].id,
                "failed",
                1.0,
                Some("synthetic_failure"),
                timestamp,
            )?;
            storage::publish_ready_replacement(&tx, &children[1].id)?;
            assert_eq!(
                storage::list_transcript_segments(&tx, &f.chunk.recording_id)?
                    .iter()
                    .map(|s| s.text.as_str())
                    .collect::<Vec<_>>(),
                vec!["old", "spanning utterance"]
            );
            storage::update_processing_chunk_status(
                &tx,
                &children[1].id,
                "succeeded",
                1.0,
                None,
                timestamp + 1,
            )?;
            storage::publish_ready_replacement(&tx, &children[1].id)?;
            let visible = storage::list_transcript_segments(&tx, &f.chunk.recording_id)?;
            assert_eq!(visible.len(), 3);
            assert!(visible.iter().any(|s| s.text == "old"));
            assert!(!visible.iter().any(|s| s.text == "spanning utterance"));
            tx.commit()?;
            Ok(())
        })
        .unwrap();
}
