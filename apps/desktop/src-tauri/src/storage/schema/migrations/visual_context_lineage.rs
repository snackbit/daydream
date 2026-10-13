use rusqlite_migration::M;

pub(in crate::storage) fn migrations() -> Vec<M<'static>> {
    // Old rolling-window starts cannot prove where inherited descriptions began.
    // Fail closed for historical output; fresh processing writes precise lineage.
    vec![M::up(
        "UPDATE studio_visual_samples SET context_start_ms=MIN(context_start_ms,0) WHERE observation_json IS NOT NULL;
         UPDATE vlm_observations SET context_start_ms=MIN(COALESCE(context_start_ms,frame_timestamp_ms),0) WHERE prompt_version='screen-changes-v1';",
    )]
}

#[cfg(test)]
mod tests {
    use crate::storage;
    use std::path::PathBuf;

    #[test]
    fn historical_visual_lineage_is_conservative_without_removing_recordings_or_ocr() {
        let storage = storage::test_storage(PathBuf::new());
        storage.with_connection_mut(|connection| {
            storage::create_recording(connection, &storage::CreateRecordingInput { id: "00000000-0000-7000-8000-000000000001".into(), active_scene_id: None, created_at_ms: 1, default_fps: 3.0, notes: None, output_root: None, quality_label: "balanced-ocr".into(), storage_dir: "/tmp/synthetic".into() })?;
            storage::create_processing_job(connection, &storage::CreateProcessingJobInput { id: "00000000-0000-7000-8000-000000000002".into(), recording_id: "00000000-0000-7000-8000-000000000001".into(), kind: "ai-processing".into(), status: "running".into(), error: None, created_at_ms: 1, updated_at_ms: 1 })?;
            let chunk = storage::create_processing_chunk(connection, &storage::CreateProcessingChunkInput { id: "00000000-0000-7000-8000-000000000003".into(), job_id: "00000000-0000-7000-8000-000000000002".into(), recording_id: "00000000-0000-7000-8000-000000000001".into(), segment_id: None, track_id: None, source_kind: "vlm".into(), status: "running".into(), start_offset_ms: Some(0), end_offset_ms: Some(10_000), progress: 0.0, attempt_count: 1, error_code: None, result_code: None, created_at_ms: 1, updated_at_ms: 1 })?;
            for (id, version) in [("00000000-0000-7000-8000-000000000004", "screen-changes-v1"), ("00000000-0000-7000-8000-000000000005", "legacy-vlm")] {
                storage::create_vlm_observation(connection, &storage::CreateVisualRegionInput { id: id.into(), job_id: chunk.job_id.clone(), recording_id: chunk.recording_id.clone(), chunk_id: chunk.id.clone(), frame_timestamp_ms: 5000, source_frame_ref: "synthetic-frame".into(), region_x: 0, region_y: 0, region_width: 64, region_height: 32, text: "Synthetic description".into(), confidence: None, model: None, provider: "synthetic".into(), prompt_version: Some(version.into()), observation_kind: Some("screen-observation".into()) })?;
            }
            connection.execute("INSERT INTO studio_visual_samples(job_id,recording_id,source_id,frame_ms,end_ms,fingerprint,pixel_hash,chunk_id,ocr_json,observation_json,context_start_ms,width,height,source_frame_ref) VALUES(?1,?2,'screen',5000,6000,'synthetic-model','synthetic-pixels',?3,'synthetic-ocr','synthetic-description',2000,64,32,'synthetic-frame')", rusqlite::params![chunk.job_id,chunk.recording_id,chunk.id])?;
            let version: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
            connection.pragma_update(None, "user_version", version - 1)?;
            super::super::apply_migrations(connection).unwrap();
            let row: (i64, String, String) = connection.query_row("SELECT context_start_ms,ocr_json,observation_json FROM studio_visual_samples", [], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?)))?;
            assert_eq!(row, (0, "synthetic-ocr".into(), "synthetic-description".into()));
            assert!(storage::get_recording(connection, &chunk.recording_id).is_ok());
            let old: Option<i64> = connection.query_row("SELECT context_start_ms FROM vlm_observations WHERE id='00000000-0000-7000-8000-000000000004'", [], |row| row.get(0))?;
            let independent: Option<i64> = connection.query_row("SELECT context_start_ms FROM vlm_observations WHERE id='00000000-0000-7000-8000-000000000005'", [], |row| row.get(0))?;
            assert_eq!(old, Some(0));
            assert_eq!(independent, None);
            let migrated: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
            assert_eq!(migrated, version);
            Ok(())
        }).unwrap();
    }
}
