use super::*;

fn current(time: i64) -> FullFrame {
    FullFrame {
        path: PathBuf::new(),
        timestamp_ms: time,
        end_ms: time + 1000,
        width: 64,
        height: 32,
        pixel_hash: "synthetic-current".into(),
        source_ref: "synthetic-frame".into(),
    }
}

fn chain(connection: &rusqlite::Connection, chunk: &ProcessingChunkRow) {
    for time in [0, 1000, 2000, 3000, 4000] {
        connection.execute("INSERT INTO studio_visual_samples(job_id,recording_id,source_id,frame_ms,end_ms,fingerprint,pixel_hash,chunk_id,observation_json,context_start_ms,width,height,source_frame_ref) VALUES(?1,?2,'screen',?3,?4,'model','synthetic',?5,?6,0,64,32,'synthetic-frame')", rusqlite::params![chunk.job_id,chunk.recording_id,time,time+1000,chunk.id,r#"{"status":"changed","text":"Synthetic dependent description"}"#]).unwrap();
    }
}

#[test]
fn transitive_lineage_survives_after_its_original_frame_leaves_the_window() {
    let storage = storage::test_storage(PathBuf::new());
    storage
        .with_connection(|connection| {
            let (chunk, _) = crate::ai::tests::test_transcription_chunk(connection);
            chain(connection, &chunk);
            let rows = samples(connection, &chunk, &current(5000), "screen", "model")?;
            assert_eq!(
                rows.iter().map(|row| row.time).collect::<Vec<_>>(),
                vec![2000, 3000, 4000]
            );
            assert_eq!(dependency_start(&rows, 5000), 0);
            // The original frame is outside all three directly supplied images.
            storage::create_redaction(
                connection,
                &storage::CreateRedactionInput {
                    id: Uuid::now_v7().to_string(),
                    recording_id: chunk.recording_id.clone(),
                    start_offset_ms: 0,
                    end_offset_ms: 1000,
                    track_ids_json: r#"["screen"]"#.into(),
                    reason: None,
                    created_at_ms: 1,
                },
            )?;
            assert!(samples(connection, &chunk, &current(5000), "screen", "model")?.is_empty());
            assert!(!eligible(
                connection,
                &chunk.recording_id,
                "screen",
                0,
                6000
            )?);
            assert!(eligible(
                connection,
                &chunk.recording_id,
                "other-screen",
                0,
                6000
            )?);
            Ok(())
        })
        .unwrap();
}

#[test]
fn context_restarts_at_a_safe_boundary_and_rejects_gaps_and_other_models() {
    let storage = storage::test_storage(PathBuf::new());
    storage
        .with_connection(|connection| {
            let (chunk, _) = crate::ai::tests::test_transcription_chunk(connection);
            chain(connection, &chunk);
            connection.execute(
                "UPDATE studio_visual_samples SET context_start_ms=3000 WHERE frame_ms>=3000",
                [],
            )?;
            storage::create_redaction(
                connection,
                &storage::CreateRedactionInput {
                    id: Uuid::now_v7().to_string(),
                    recording_id: chunk.recording_id.clone(),
                    start_offset_ms: 0,
                    end_offset_ms: 1000,
                    track_ids_json: "[]".into(),
                    reason: None,
                    created_at_ms: 1,
                },
            )?;
            let rows = samples(connection, &chunk, &current(5000), "screen", "model")?;
            assert_eq!(
                rows.iter().map(|row| row.time).collect::<Vec<_>>(),
                vec![3000, 4000]
            );
            assert_eq!(dependency_start(&rows, 5000), 3000);
            assert!(samples(connection, &chunk, &current(6000), "screen", "model")?.is_empty());
            assert!(
                samples(connection, &chunk, &current(5000), "screen", "other-model")?.is_empty()
            );
            assert_eq!(dependency_start(&[], 5000), 5000);
            Ok(())
        })
        .unwrap();
}
