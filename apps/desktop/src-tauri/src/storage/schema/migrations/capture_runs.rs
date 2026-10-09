use rusqlite_migration::M;
pub(in crate::storage) fn migrations() -> Vec<M<'static>> {
    vec![M::up(
        "ALTER TABLE recording_segments ADD COLUMN capture_run_id TEXT;",
    )]
}
