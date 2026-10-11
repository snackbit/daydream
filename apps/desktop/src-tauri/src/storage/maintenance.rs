//! Complete library cleanup, including data without recording foreign keys.
use rusqlite::{Connection, Transaction};
const RETENTION_RESET_PENDING_KEY: &str = "maintenance.retention-reset-pending";
pub(crate) fn pause_retention_after_reset(connection: &Connection) -> rusqlite::Result<()> {
    crate::storage::save_app_setting(
        connection,
        &crate::storage::SaveAppSettingInput {
            key: RETENTION_RESET_PENDING_KEY.into(),
            value_json: "true".into(),
            is_sensitive: false,
            timestamp_ms: chrono::Utc::now().timestamp_millis(),
        },
    )?;
    Ok(())
}
pub(crate) fn retention_cleanup_paused(connection: &Connection) -> rusqlite::Result<bool> {
    connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM app_settings WHERE key=?1 AND value_json='true')",
        [RETENTION_RESET_PENDING_KEY],
        |row| row.get(0),
    )
}
pub(crate) fn authorize_retention_on_settings_save(
    connection: &Connection,
) -> rusqlite::Result<()> {
    connection.execute(
        "DELETE FROM app_settings WHERE key=?1",
        [RETENTION_RESET_PENDING_KEY],
    )?;
    Ok(())
}
pub(crate) const LIBRARY_TABLES: &[&str] = &[
    "recall_generation_inputs",
    "recall_generation_windows",
    "recall_generations",
    "recall_feedback",
    "recall_tasks",
    "recall_day_reviews",
    "recall_session_corrections",
    "recall_jobs",
    "recall_index",
    "recall_evidence",
    "recall_source_lineage",
    "recall_source_files",
    "processing_audio_ranges",
    "processing_chunk_runtime",
    "processing_job_runtime",
    "processing_job_ocr_configs",
    "ocr_frame_occurrences",
    "ocr_document_contents",
    "visual_frame_processing_states",
    "frame_similarity_entries",
    "vlm_observations",
    "ocr_regions",
    "frame_activity_intervals",
    "transcript_words",
    "transcript_language_context",
    "transcript_segments",
    "processing_chunks",
    "processing_jobs",
    "recording_live_state",
    "recording_protection_spans",
    "capture_events",
    "redactions",
    "recording_segment_videos",
    "recording_video_sources",
    "recording_tracks",
    "recording_segments",
    "recordings",
    "notifications",
    "recording_recovery_operations",
];
pub(crate) fn library_counts(connection: &Connection) -> rusqlite::Result<(u64, u64, u64)> {
    let recordings = count_rows(connection, "recordings")?;
    let models = count_rows(connection, "ai_model_assets")?;
    let mut total = 0_u64;
    for table in LIBRARY_TABLES {
        total = total
            .checked_add(count_rows(connection, table)?)
            .ok_or(rusqlite::Error::InvalidQuery)?;
    }
    Ok((recordings, total, models))
}
fn count_rows(connection: &Connection, table: &str) -> rusqlite::Result<u64> {
    let count: i64 = connection.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
        row.get(0)
    })?;
    u64::try_from(count).map_err(|_| rusqlite::Error::InvalidQuery)
}
pub(crate) fn clear_library(transaction: &Transaction<'_>, factory: bool) -> rusqlite::Result<()> {
    for table in LIBRARY_TABLES {
        transaction.execute(&format!("DELETE FROM {table}"), [])?;
    }
    transaction.execute("DELETE FROM recall_fts", [])?;
    transaction.execute("DELETE FROM recall_dirty_recordings", [])?;
    transaction.execute("UPDATE recall_meta SET revision=revision+1,privacy_revision=privacy_revision+1,source_dirty=1 WHERE id=1", [])?;
    if factory {
        transaction.execute("DELETE FROM ai_model_assets", [])?;
    }
    Ok(())
}
/// SQLite stays open with its schema and operation ledger. Reclaim freed pages
/// and checkpoint sensitive older WAL frames before reporting deletion complete.
pub(crate) fn compact_after_cleanup(connection: &Connection) -> rusqlite::Result<()> {
    connection.execute_batch("VACUUM;")?;
    let (busy, remaining, checkpointed): (i64, i64, i64) =
        connection.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })?;
    if busy != 0 || (remaining >= 0 && checkpointed < remaining) {
        return Err(rusqlite::Error::InvalidQuery);
    }
    Ok(())
}
pub(crate) fn reset_preferences(transaction: &Transaction<'_>) -> rusqlite::Result<()> {
    transaction.execute("DELETE FROM app_settings", [])?;
    transaction.execute("DELETE FROM recorder_portal_restore_tokens", [])?;
    transaction.execute("DELETE FROM recorder_scene_sources", [])?;
    transaction.execute("DELETE FROM recorder_scenes", [])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cleanup_compacts_private_database_pages_and_truncates_wal() {
        let root = std::env::temp_dir().join(format!(
            "daydream-maintenance-compact-{}",
            uuid::Uuid::now_v7()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("daydream.sqlite3");
        let storage = crate::storage::test_file_storage(path.clone());
        let private = "SYNTHETIC-CAPTURED-PRIVATE-TEXT".repeat(4096);
        storage
            .with_connection_mut(|connection| {
                connection.execute(
                    "INSERT INTO recall_feedback VALUES('2026-01-01','target','up',?1)",
                    [&private],
                )?;
                connection.pragma_update(None, "secure_delete", "ON")?;
                let transaction = connection.transaction()?;
                clear_library(&transaction, false)?;
                transaction.commit()?;
                compact_after_cleanup(connection)?;
                Ok(())
            })
            .unwrap();
        let data = std::fs::read(&path).unwrap();
        let needle = b"SYNTHETIC-CAPTURED-PRIVATE-TEXT";
        assert!(!data.windows(needle.len()).any(|window| window == needle));
        assert_eq!(
            std::fs::metadata(root.join("daydream.sqlite3-wal"))
                .unwrap()
                .len(),
            0
        );
        drop(storage);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn clear_library_removes_unlinked_manual_data_and_preserves_preferences_models() {
        let storage = crate::storage::test_storage("/tmp/synthetic-maintenance-db".into());
        storage.with_connection_mut(|connection| {
            connection.execute_batch("INSERT INTO recall_tasks VALUES('task','2026-01-01','action','{}','Synthetic',NULL,'manual note','accepted',1); INSERT INTO recall_feedback VALUES('2026-01-01','target','up','manual correction'); INSERT INTO recall_session_corrections VALUES('2026-01-01','session','manual','title','category',1); INSERT INTO app_settings(key,value_json,is_sensitive,created_at_ms,updated_at_ms) VALUES('synthetic.preference','true',0,1,1);")?;
            let transaction=connection.transaction()?; clear_library(&transaction,false)?; transaction.commit()?;
            assert_eq!(library_counts(connection)?.1,0);
            assert_eq!(connection.query_row("SELECT COUNT(*) FROM recall_fts",[],|row|row.get::<_,i64>(0))?,0);
            assert_eq!(connection.query_row("SELECT COUNT(*) FROM recall_dirty_recordings",[],|row|row.get::<_,i64>(0))?,0);
            assert_eq!(connection.query_row("SELECT COUNT(*) FROM app_settings",[],|row|row.get::<_,i64>(0))?,1);
            Ok(())
        }).unwrap();
    }
    #[test]
    fn database_failure_rolls_back_metadata_without_claiming_files_restored() {
        let storage = crate::storage::test_storage("/tmp/synthetic-maintenance-failure".into());
        storage.with_connection_mut(|connection| {
            connection.execute_batch("INSERT INTO recall_feedback VALUES('2026-01-01','target','up','synthetic'); CREATE TRIGGER reject_cleanup BEFORE DELETE ON recall_feedback BEGIN SELECT RAISE(FAIL,'synthetic cleanup failure'); END;")?;
            { let transaction=connection.transaction()?; assert!(clear_library(&transaction,false).is_err()); }
            assert_eq!(connection.query_row("SELECT COUNT(*) FROM recall_feedback",[],|row|row.get::<_,i64>(0))?,1);
            Ok(())
        }).unwrap();
    }
}
