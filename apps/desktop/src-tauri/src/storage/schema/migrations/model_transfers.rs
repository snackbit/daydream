use rusqlite_migration::M;

pub(in crate::storage) fn migrations() -> Vec<M<'static>> {
    vec![M::up(MODEL_TRANSFERS_SQL)]
}

pub(in crate::storage) const MODEL_TRANSFERS_SQL: &str = r#"
CREATE TABLE ai_model_assets_transfer (
 id TEXT PRIMARY KEY NOT NULL,
 provider TEXT NOT NULL CHECK(provider IN ('local-whisper-cpp','openai-compatible','local-tesseract-ocr','local-embedding')),
 display_label TEXT NOT NULL, local_path TEXT,
 size_bytes INTEGER CHECK(size_bytes IS NULL OR size_bytes >= 0),
 checksum_algorithm TEXT, checksum TEXT, etag TEXT, source_url TEXT,
 downloaded_bytes INTEGER NOT NULL DEFAULT 0 CHECK(downloaded_bytes >= 0),
 total_bytes INTEGER CHECK(total_bytes IS NULL OR total_bytes >= 0), error_code TEXT,
 verification_status TEXT NOT NULL DEFAULT 'unknown' CHECK(verification_status IN ('unknown','pending','passed','failed')),
 download_status TEXT NOT NULL CHECK(download_status IN ('missing','queued','downloading','paused','available','failed','cancelled')),
 self_test_status TEXT NOT NULL CHECK(self_test_status IN ('not-run','running','passed','failed')),
 created_at_ms INTEGER NOT NULL CHECK(created_at_ms >= 0),
 updated_at_ms INTEGER NOT NULL CHECK(updated_at_ms >= created_at_ms)
);
INSERT INTO ai_model_assets_transfer (id,provider,display_label,local_path,size_bytes,checksum_algorithm,checksum,etag,source_url,downloaded_bytes,total_bytes,error_code,verification_status,download_status,self_test_status,created_at_ms,updated_at_ms)
 SELECT id,provider,display_label,local_path,size_bytes,checksum_algorithm,checksum,etag,source_url,downloaded_bytes,total_bytes,error_code,verification_status,download_status,self_test_status,created_at_ms,updated_at_ms FROM ai_model_assets;
DROP TABLE ai_model_assets;
ALTER TABLE ai_model_assets_transfer RENAME TO ai_model_assets;
CREATE TABLE ai_model_download_runtime (
 model_id TEXT PRIMARY KEY NOT NULL REFERENCES ai_model_assets(id) ON DELETE CASCADE,
 attempt INTEGER NOT NULL DEFAULT 1 CHECK(attempt > 0),
 phase TEXT NOT NULL DEFAULT 'queued',
 control TEXT NOT NULL DEFAULT 'run' CHECK(control IN ('run','pause','cancel')),
 queue_order INTEGER NOT NULL DEFAULT 0,
 active_elapsed_ms INTEGER NOT NULL DEFAULT 0 CHECK(active_elapsed_ms >= 0)
);
"#;

#[cfg(test)]
mod tests {
    #[test]
    fn transfer_migration_preserves_legacy_assets_and_admits_embedding_pause() {
        let mut connection = rusqlite::Connection::open_in_memory().unwrap();
        super::super::migrations()
            .to_version(&mut connection, 25)
            .unwrap();
        connection.execute("INSERT INTO ai_model_assets(id,provider,display_label,download_status,self_test_status,created_at_ms,updated_at_ms,downloaded_bytes,etag,error_code) VALUES ('legacy','local-whisper-cpp','Synthetic','cancelled','not-run',1,2,7,'validator','saved-error')", []).unwrap();
        super::super::migrations()
            .to_latest(&mut connection)
            .unwrap();
        let saved = connection.query_row("SELECT provider,download_status,downloaded_bytes,etag,error_code FROM ai_model_assets WHERE id='legacy'",[],|row|Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?,row.get::<_,i64>(2)?,row.get::<_,String>(3)?,row.get::<_,String>(4)?))).unwrap();
        assert_eq!(
            saved,
            (
                "local-whisper-cpp".into(),
                "cancelled".into(),
                7,
                "validator".into(),
                "saved-error".into()
            )
        );
        for id in ["multilingual-e5-small-q8_0", "bge-small-en-v1.5-q8_0"] {
            connection.execute("INSERT INTO ai_model_assets(id,provider,display_label,download_status,self_test_status,created_at_ms,updated_at_ms) VALUES (?1,'local-embedding','Synthetic','paused','not-run',3,3)",[id]).unwrap();
        }
        assert!(connection
            .execute("UPDATE ai_model_assets SET provider='invalid'", [])
            .is_err());
    }
}
