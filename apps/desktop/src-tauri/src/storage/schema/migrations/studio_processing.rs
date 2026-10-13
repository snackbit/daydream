use rusqlite_migration::M;

pub(in crate::storage) fn migrations() -> Vec<M<'static>> {
    vec![M::up_with_hook("", |transaction| {
        let views = {
            let mut query = transaction.prepare("SELECT type,name,sql FROM sqlite_master WHERE type IN('view','trigger') AND sql IS NOT NULL")?;
            let rows = query
                .query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            rows
        };
        for (kind, name, _) in &views {
            transaction.execute_batch(&format!(
                "DROP {} \"{}\"",
                if kind == "view" { "VIEW" } else { "TRIGGER" },
                name.replace('"', "\"\"")
            ))?;
        }
        // Rebuild in place, preserving every existing column and index. Foreign
        // keys are disabled by the migration owner and checked before commit.
        for table in ["processing_jobs", "processing_chunks"] {
            let definition: String = transaction.query_row(
                "SELECT sql FROM sqlite_master WHERE type='table' AND name=?1",
                [table],
                |row| row.get(0),
            )?;
            let indexes = {
                let mut query = transaction.prepare("SELECT sql FROM sqlite_master WHERE type='index' AND tbl_name=?1 AND sql IS NOT NULL")?;
                let rows = query
                    .query_map([table], |row| row.get::<_, String>(0))?
                    .collect::<rusqlite::Result<Vec<_>>>()?;
                rows
            };
            let temporary = format!("{table}_studio");
            let expanded = definition
                .replacen(table, &temporary, 1)
                .replace("'vlm'", "'vlm','waveform'");
            transaction.execute_batch(&expanded)?;
            transaction.execute_batch(&format!("INSERT INTO {temporary} SELECT * FROM {table}; DROP TABLE {table}; ALTER TABLE {temporary} RENAME TO {table};"))?;
            for index in indexes {
                transaction.execute_batch(&index)?;
            }
        }
        for (_, _, definition) in views {
            transaction.execute_batch(&definition)?;
        }
        // Waveform progress changes no searchable or reviewable evidence.
        // Keep its cache jobs out of Recall's revision/invalidation hot path.
        for table in ["processing_chunks", "processing_jobs"] {
            let column = if table == "processing_chunks" {
                "source_kind"
            } else {
                "kind"
            };
            for action in ["INSERT", "UPDATE", "DELETE"] {
                let name = format!("recall_dirty_{table}_{action}");
                let definition: String = transaction.query_row(
                    "SELECT sql FROM sqlite_master WHERE type='trigger' AND name=?1",
                    [&name],
                    |row| row.get(0),
                )?;
                transaction.execute_batch(&format!("DROP TRIGGER {name}"))?;
                let condition = if action == "DELETE" {
                    format!("OLD.{column}!='waveform'")
                } else if action == "UPDATE" {
                    format!("NEW.{column}!='waveform' OR OLD.{column}!='waveform'")
                } else {
                    format!("NEW.{column}!='waveform'")
                };
                transaction.execute_batch(&definition.replacen(
                    " BEGIN",
                    &format!(" WHEN {condition} BEGIN"),
                    1,
                ))?;
            }
        }
        transaction.execute_batch("CREATE TABLE waveform_sources (
          source_key TEXT PRIMARY KEY, recording_id TEXT NOT NULL REFERENCES recordings(id) ON DELETE CASCADE,
          chunk_id TEXT NOT NULL REFERENCES processing_chunks(id) ON DELETE CASCADE
        ); CREATE INDEX idx_waveform_sources_recording ON waveform_sources(recording_id);")?;
        transaction.execute_batch(
            "CREATE TABLE waveform_admission (
          recording_id TEXT PRIMARY KEY REFERENCES recordings(id) ON DELETE CASCADE,
          generation INTEGER NOT NULL DEFAULT 1
        ); INSERT INTO waveform_admission(recording_id) SELECT id FROM recordings;",
        )?;
        for (name, table, action, condition, value) in [
            (
                "waveform_new_recording",
                "recordings",
                "INSERT",
                "",
                "NEW.id",
            ),
            (
                "waveform_recording_status",
                "recordings",
                "UPDATE OF status",
                "",
                "NEW.id",
            ),
            (
                "waveform_fragment_closed",
                "recording_segments",
                "UPDATE OF state",
                "WHEN NEW.state='finalized'",
                "NEW.recording_id",
            ),
            (
                "waveform_finalized_insert",
                "recording_segments",
                "INSERT",
                "WHEN NEW.state='finalized'",
                "NEW.recording_id",
            ),
            (
                "waveform_cleanup_changed",
                "recording_audio_cleanup",
                "UPDATE OF active_revision",
                "",
                "NEW.recording_id",
            ),
            (
                "waveform_cleanup_created",
                "recording_audio_cleanup",
                "INSERT",
                "",
                "NEW.recording_id",
            ),
            (
                "waveform_redaction_insert",
                "redactions",
                "INSERT",
                "",
                "NEW.recording_id",
            ),
            (
                "waveform_redaction_update",
                "redactions",
                "UPDATE",
                "",
                "NEW.recording_id",
            ),
            (
                "waveform_redaction_delete",
                "redactions",
                "DELETE",
                "",
                "OLD.recording_id",
            ),
            ("waveform_live_classified", "recording_live_state", "UPDATE OF classified_through_ms", "WHEN EXISTS(SELECT 1 FROM recording_segments s JOIN recordings r ON r.id=s.recording_id WHERE s.recording_id=NEW.recording_id AND s.state='finalized' AND COALESCE(s.timeline_base_ms,s.started_at_ms-COALESCE(r.started_at_ms,s.started_at_ms))+s.duration_ms>OLD.classified_through_ms AND COALESCE(s.timeline_base_ms,s.started_at_ms-COALESCE(r.started_at_ms,s.started_at_ms))+s.duration_ms<=NEW.classified_through_ms)", "NEW.recording_id"),
            ("waveform_protection_materialized", "recording_protection_spans", "UPDATE OF status", "WHEN NEW.status='materialized'", "NEW.recording_id"),
        ] {
            transaction.execute_batch(&format!("CREATE TRIGGER {name} AFTER {action} ON {table} {condition} BEGIN INSERT INTO waveform_admission(recording_id) SELECT id FROM recordings WHERE id={value} ON CONFLICT(recording_id) DO UPDATE SET generation=generation+1; END;"))?;
        }
        transaction.execute_batch("ALTER TABLE processing_job_runtime ADD COLUMN visual_protocol TEXT NOT NULL DEFAULT 'legacy';
          CREATE TABLE studio_visual_samples (
            job_id TEXT NOT NULL REFERENCES processing_jobs(id) ON DELETE CASCADE,
            recording_id TEXT NOT NULL REFERENCES recordings(id) ON DELETE CASCADE,
            source_id TEXT NOT NULL, frame_ms INTEGER NOT NULL, end_ms INTEGER NOT NULL,
            fingerprint TEXT NOT NULL, pixel_hash TEXT NOT NULL, frame_path TEXT,
            width INTEGER NOT NULL, height INTEGER NOT NULL, source_frame_ref TEXT NOT NULL,
            chunk_id TEXT NOT NULL REFERENCES processing_chunks(id) ON DELETE CASCADE,
            ocr_json TEXT, observation_json TEXT, context_start_ms INTEGER NOT NULL,
            PRIMARY KEY(job_id,source_id,frame_ms,fingerprint)
          );
          CREATE INDEX idx_studio_visual_context ON studio_visual_samples(recording_id,source_id,frame_ms);
          ALTER TABLE vlm_observations ADD COLUMN context_start_ms INTEGER;
          ALTER TABLE vlm_observations ADD COLUMN confirmed_end_ms INTEGER;")?;
        let mut check = transaction.prepare("PRAGMA foreign_key_check")?;
        if check.query([])?.next()?.is_some() {
            return Err(rusqlite::Error::InvalidQuery.into());
        }
        Ok(())
    })]
}
