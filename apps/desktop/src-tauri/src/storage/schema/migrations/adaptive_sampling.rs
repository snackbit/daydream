use rusqlite_migration::M;

pub(in crate::storage) fn migrations() -> Vec<M<'static>> {
    vec![M::up(
        r#"
INSERT INTO app_settings(key,value_json,is_sensitive,created_at_ms,updated_at_ms)
SELECT 'ai.processing.visualSamplingMode','"full"',0,CAST(unixepoch('subsec')*1000 AS INTEGER),CAST(unixepoch('subsec')*1000 AS INTEGER)
WHERE EXISTS(SELECT 1 FROM app_settings)
ON CONFLICT(key) DO NOTHING;
CREATE INDEX IF NOT EXISTS idx_recall_job_execution_admission ON recall_jobs(kind,config_fingerprint,created_at_ms DESC);
ALTER TABLE processing_job_runtime ADD COLUMN visual_recovery_json TEXT CHECK(visual_recovery_json IS NULL OR json_valid(visual_recovery_json));
"#,
    )]
}

#[cfg(test)]
mod tests {
    #[test]
    fn upgrade_preserves_existing_full_or_explicit_adaptive_policy() {
        for existing in [None, Some("full"), Some("adaptive-v1")] {
            let mut connection = rusqlite::Connection::open_in_memory().unwrap();
            crate::storage::migrations()
                .to_version(&mut connection, 39)
                .unwrap();
            connection.execute("INSERT INTO app_settings(key,value_json,is_sensitive,created_at_ms,updated_at_ms) VALUES('record.fps','0.2',0,1,1)",[]).unwrap();
            if let Some(policy) = existing {
                connection.execute("INSERT INTO app_settings(key,value_json,is_sensitive,created_at_ms,updated_at_ms) VALUES('ai.processing.visualSamplingMode',?1,0,1,1)",[serde_json::to_string(policy).unwrap()]).unwrap();
            }
            crate::storage::migrations()
                .to_latest(&mut connection)
                .unwrap();
            let policy:String=connection.query_row("SELECT value_json FROM app_settings WHERE key='ai.processing.visualSamplingMode'",[],|row|row.get(0)).unwrap();
            assert_eq!(
                policy,
                serde_json::to_string(existing.unwrap_or("full")).unwrap()
            );
        }
    }
    #[test]
    fn empty_install_uses_the_new_default_without_changing_old_job_snapshots() {
        let mut connection = rusqlite::Connection::open_in_memory().unwrap();
        crate::storage::migrations()
            .to_version(&mut connection, 39)
            .unwrap();
        crate::storage::migrations()
            .to_latest(&mut connection)
            .unwrap();
        assert_eq!(
            connection
                .query_row("SELECT COUNT(*) FROM app_settings", [], |row| row
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
        assert_eq!(
            crate::settings::default_settings()
                .ai
                .processing
                .visual_sampling_mode,
            "adaptive-v1"
        );
    }
}
