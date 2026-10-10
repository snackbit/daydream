use super::*;

#[test]
fn reprocessing_history_retains_text_outside_scoped_audio_and_assigns_each_segment_once() {
    let f = fixture();
    f.storage
        .with_connection(|c| {
            speech(c, &f.chunk, &f.track, "later text", 7000, 8000)?;
            Ok(())
        })
        .unwrap();
    let job = start_preview(&f, request(&f));
    for status in ["queued", "failed", "succeeded"] {
        f.storage
            .with_connection_mut(|c| {
                let tx = c.transaction()?;
                let child = storage::list_processing_chunks(&tx, &job.id)?.remove(0);
                storage::update_processing_chunk_status(
                    &tx,
                    &child.id,
                    status,
                    if status == "queued" { 0.0 } else { 1.0 },
                    None,
                    child.created_at_ms + 1,
                )?;
                if status == "succeeded" {
                    speech(&tx, &child, &f.track, "replacement", 1000, 3000)?;
                    storage::publish_ready_replacement(&tx, &child.id)?;
                }
                tx.commit()?;
                Ok(())
            })
            .unwrap();
        let state = processing_state_for_recording(&f.storage, &f.chunk.recording_id).unwrap();
        let microphone = state
            .transcription_tracks
            .iter()
            .find(|t| t.source_role == "microphone")
            .unwrap();
        let texts = microphone
            .chunks
            .iter()
            .flat_map(|c| &c.transcript_segments)
            .map(|s| s.text.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            texts.iter().filter(|t| **t == "later text").count(),
            1,
            "out-of-range text disappeared during {status}"
        );
        assert_eq!(
            texts
                .iter()
                .filter(|t| **t
                    == if status == "succeeded" {
                        "replacement"
                    } else {
                        "old"
                    })
                .count(),
            1,
            "overlapping history must not duplicate text"
        );
    }
}

#[test]
fn reprocessing_audio_retry_survives_a_newer_cancelled_unpublished_pass() {
    let f = fixture();
    let earlier = start_preview(&f, request(&f));
    f.storage
        .with_connection(|c| {
            let child = storage::list_processing_chunks(c, &earlier.id)?.remove(0);
            storage::update_processing_chunk_status(
                c,
                &child.id,
                "failed",
                0.0,
                Some("synthetic_failure"),
                child.created_at_ms + 1,
            )?;
            storage::update_processing_job_status(
                c,
                &earlier.id,
                "failed",
                None,
                child.created_at_ms + 1,
            )?;
            Ok(())
        })
        .unwrap();
    let newer = start_preview(&f, request(&f));
    update_processing_job_for_action(&f.storage, &newer.id, "cancelled").unwrap();
    retry_processing_job_for_action(&f.storage, &earlier.id).unwrap();
    f.storage
        .with_connection_mut(|c| {
            let tx = c.transaction()?;
            let child = storage::list_processing_chunks(&tx, &earlier.id)?.remove(0);
            speech(&tx, &child, &f.track, "recovered audio", 1000, 3000)?;
            storage::update_processing_chunk_status(
                &tx,
                &child.id,
                "succeeded",
                1.0,
                None,
                child.created_at_ms + 2,
            )?;
            storage::publish_ready_replacement(&tx, &child.id)?;
            assert_eq!(
                storage::list_transcript_segments(&tx, &child.recording_id)?
                    .iter()
                    .map(|s| s.text.as_str())
                    .collect::<Vec<_>>(),
                vec!["recovered audio"]
            );
            tx.commit()?;
            Ok(())
        })
        .unwrap();
}
