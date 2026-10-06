use rusqlite_migration::M;
pub(in crate::storage) fn migrations() -> Vec<M<'static>> {
    vec![M::up(
        r#"
ALTER TABLE processing_chunk_runtime ADD COLUMN config_json TEXT CHECK(config_json IS NULL OR json_valid(config_json));
ALTER TABLE processing_chunk_runtime ADD COLUMN selection_json TEXT CHECK(selection_json IS NULL OR json_valid(selection_json));
ALTER TABLE processing_chunk_runtime ADD COLUMN total_units INTEGER CHECK(total_units IS NULL OR total_units>=0);
ALTER TABLE processing_audio_ranges ADD COLUMN updated_at_ms INTEGER NOT NULL DEFAULT 0;
CREATE INDEX idx_processing_audio_ranges_status ON processing_audio_ranges(chunk_id,status,updated_at_ms);
"#,
    )]
}
