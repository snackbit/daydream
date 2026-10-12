use rusqlite_migration::M;

pub(super) fn migrations() -> Vec<M<'static>> {
    vec![M::up(
        r#"
-- Rebuild old snapshots: crossing speech was omitted and silent publication
-- could leave hidden history in the cache before this integration fix.
INSERT OR IGNORE INTO recall_dirty_recordings SELECT id FROM recordings;
UPDATE recall_meta SET revision=revision+1,source_dirty=1 WHERE id=1;

CREATE TRIGGER speech_publication_recall_dirty AFTER UPDATE OF status ON processing_speech_units
WHEN OLD.status!=NEW.status AND (OLD.status='published' OR NEW.status='published')
BEGIN
 INSERT OR IGNORE INTO recall_dirty_recordings SELECT recording_id FROM processing_jobs WHERE id=NEW.job_id;
 UPDATE recall_meta SET revision=revision+1,source_dirty=1 WHERE id=1;
END;
CREATE TRIGGER speech_insert_recall_dirty AFTER INSERT ON processing_speech_units
WHEN NEW.status='published'
BEGIN
 INSERT OR IGNORE INTO recall_dirty_recordings SELECT recording_id FROM processing_jobs WHERE id=NEW.job_id;
 UPDATE recall_meta SET revision=revision+1,source_dirty=1 WHERE id=1;
END;
CREATE TRIGGER speech_delete_recall_dirty AFTER DELETE ON processing_speech_units
WHEN OLD.status='published'
BEGIN
 INSERT OR IGNORE INTO recall_dirty_recordings SELECT recording_id FROM processing_jobs WHERE id=OLD.job_id;
 UPDATE recall_meta SET revision=revision+1,source_dirty=1 WHERE id=1;
END;
"#,
    )]
}

#[cfg(test)]
mod tests {
    #[test]
    fn upgrade_invalidates_old_clean_evidence_snapshots_without_changing_source_rows() {
        let mut db = rusqlite::Connection::open_in_memory().unwrap();
        crate::storage::schema::enable_connection_pragmas(&db).unwrap();
        let migrations = crate::storage::schema::migrations::migrations();
        // Version 33 predates this migration, regardless of later additions.
        let previous = 33;
        migrations.to_version(&mut db, previous).unwrap();
        let f = crate::storage::recall::tests::seed_recall_fixture(&db);
        crate::storage::refresh_evidence(&db).unwrap();
        db.execute("DELETE FROM recall_evidence", []).unwrap();
        // Old refreshes could finish clean after omitting cross-fragment speech.
        db.execute_batch("DELETE FROM recall_dirty_recordings; UPDATE recall_meta SET source_dirty=0 WHERE id=1;").unwrap();
        assert!(!db
            .query_row("SELECT source_dirty FROM recall_meta WHERE id=1", [], |r| r
                .get::<_, bool>(0))
            .unwrap());
        crate::storage::schema::apply_migrations(&mut db).unwrap();
        assert!(db
            .query_row("SELECT source_dirty FROM recall_meta WHERE id=1", [], |r| r
                .get::<_, bool>(0))
            .unwrap());
        crate::storage::refresh_evidence(&db).unwrap();
        assert_eq!(
            crate::storage::list_evidence_range(&db, 0, i64::MAX)
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            db.query_row(
                "SELECT count(*) FROM transcript_segments WHERE id=?1",
                [&f.transcript],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            1
        );
    }
}
