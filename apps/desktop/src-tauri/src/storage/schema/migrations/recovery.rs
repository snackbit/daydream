use rusqlite_migration::M;
pub(in crate::storage) fn migrations() -> Vec<M<'static>> {
    vec![M::up(
        r#"
    PRAGMA writable_schema=ON;
    UPDATE sqlite_schema SET sql=replace(sql,'''completed'', ''failed'', ''deleted''','''completed'', ''failed'', ''deleted'', ''recovered-partial''') WHERE type='table' AND name='recordings';
    PRAGMA schema_version=5001;
    PRAGMA writable_schema=OFF;
    CREATE TABLE maintenance_operations(id TEXT PRIMARY KEY,action TEXT NOT NULL,phase TEXT NOT NULL,error TEXT,created_at_ms INTEGER NOT NULL);
    CREATE TABLE recording_recovery_operations(id TEXT PRIMARY KEY,original_recording_id TEXT NOT NULL REFERENCES recordings(id) ON DELETE CASCADE,recovered_recording_id TEXT REFERENCES recordings(id) ON DELETE SET NULL,status TEXT NOT NULL,coverage_json TEXT NOT NULL CHECK(json_valid(coverage_json)),created_at_ms INTEGER NOT NULL);
    "#,
    )]
}

pub(in crate::storage) fn output_ownership_migrations() -> Vec<M<'static>> {
    vec![M::up(
        "ALTER TABLE recording_recovery_operations ADD COLUMN pending_output_id TEXT;",
    )]
}

#[cfg(test)]
mod tests {
    #[test]
    fn output_ownership_upgrade_preserves_legacy_recovery_rows() {
        let mut connection = rusqlite::Connection::open_in_memory().unwrap();
        super::super::migrations()
            .to_version(&mut connection, 28)
            .unwrap();
        let original = uuid::Uuid::now_v7().to_string();
        crate::storage::create_recording(
            &connection,
            &crate::storage::CreateRecordingInput {
                id: original.clone(),
                active_scene_id: None,
                created_at_ms: 1,
                default_fps: 1.0,
                notes: None,
                output_root: None,
                quality_label: "balanced-ocr".into(),
                storage_dir: "/tmp/synthetic-original".into(),
            },
        )
        .unwrap();
        connection.execute("INSERT INTO recording_recovery_operations(id,original_recording_id,status,coverage_json,created_at_ms) VALUES('legacy',?1,'failed','[]',1)", [&original]).unwrap();
        super::super::apply_migrations(&mut connection).unwrap();
        assert_eq!(connection.query_row("SELECT status,pending_output_id FROM recording_recovery_operations WHERE id='legacy'", [], |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))).unwrap(), ("failed".into(), None));
        assert!(crate::storage::recovery::pending_outputs(&connection, None)
            .unwrap()
            .is_empty());
        assert!(crate::storage::get_recording(&connection, &original).is_ok());
    }
    #[test]
    fn status_upgrade_preserves_existing_foreign_key_children() {
        let mut connection = rusqlite::Connection::open_in_memory().unwrap();
        super::super::migrations()
            .to_version(&mut connection, 26)
            .unwrap();
        let id = uuid::Uuid::now_v7().to_string();
        let segment = uuid::Uuid::now_v7().to_string();
        crate::storage::create_recording(
            &connection,
            &crate::storage::CreateRecordingInput {
                id: id.clone(),
                active_scene_id: None,
                created_at_ms: 1,
                default_fps: 1.0,
                notes: None,
                output_root: None,
                quality_label: "balanced-ocr".into(),
                storage_dir: "/tmp/synthetic-recovery".into(),
            },
        )
        .unwrap();
        crate::storage::create_recording_segment(
            &connection,
            &crate::storage::CreateRecordingSegmentInput {
                id: segment.clone(),
                recording_id: id.clone(),
                segment_index: 1,
                file_path: "/tmp/synthetic-recovery/segment.mkv".into(),
                started_at_ms: 1,
            },
        )
        .unwrap();
        super::super::apply_migrations(&mut connection).unwrap();
        connection
            .execute(
                "UPDATE recordings SET status='recovered-partial' WHERE id=?1",
                [&id],
            )
            .unwrap();
        assert_eq!(
            crate::storage::list_recording_segments(&connection, &id).unwrap()[0].id,
            segment
        );
        assert_eq!(
            connection
                .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| {
                    row.get::<_, i64>(0)
                })
                .unwrap(),
            0
        );
    }
}
