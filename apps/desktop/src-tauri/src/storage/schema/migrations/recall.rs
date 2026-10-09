use rusqlite_migration::M;

pub(in crate::storage) fn migrations() -> Vec<M<'static>> {
    vec![M::up_with_hook(SCHEMA, |transaction| {
        crate::storage::segments::backfill_segment_timeline_bases(transaction)?;
        for table in [
            "recording_segments",
            "transcript_segments",
            "ocr_frame_occurrences",
            "processing_chunks",
            "processing_jobs",
        ] {
            for operation in ["INSERT", "UPDATE", "DELETE"] {
                let alias = if operation == "DELETE" { "OLD" } else { "NEW" };
                let old = if operation == "UPDATE" {
                    "INSERT INTO recall_dirty_recordings VALUES(OLD.recording_id) ON CONFLICT DO NOTHING;"
                } else {
                    ""
                };
                let update = if table == "processing_jobs" && operation == "UPDATE" {
                    "UPDATE OF status"
                } else if table == "processing_chunks" && operation == "UPDATE" {
                    "UPDATE OF status,attempt_count,start_offset_ms,end_offset_ms"
                } else {
                    operation
                };
                transaction.execute_batch(&format!("CREATE TRIGGER recall_dirty_{table}_{operation} AFTER {update} ON {table} BEGIN INSERT INTO recall_dirty_recordings VALUES({alias}.recording_id) ON CONFLICT DO NOTHING; {old} UPDATE recall_meta SET revision=revision+1,source_dirty=1 WHERE id=1; END;"))?;
            }
        }
        Ok(())
    })]
}

const SCHEMA: &str = r#"
ALTER TABLE recording_segments ADD COLUMN timeline_base_ms INTEGER CHECK(timeline_base_ms>=0);
ALTER TABLE recording_segments ADD COLUMN timeline_basis_version INTEGER;
CREATE TABLE recall_meta(id INTEGER PRIMARY KEY CHECK(id=1),revision INTEGER NOT NULL,source_dirty INTEGER NOT NULL DEFAULT 1,privacy_revision INTEGER NOT NULL DEFAULT 0);
INSERT INTO recall_meta VALUES(1,0,1,0);
CREATE TABLE recall_dirty_recordings(recording_id TEXT PRIMARY KEY);
INSERT INTO recall_dirty_recordings SELECT id FROM recordings;
CREATE TABLE recall_source_files(segment_id TEXT PRIMARY KEY REFERENCES recording_segments(id) ON DELETE CASCADE,path TEXT NOT NULL,available INTEGER NOT NULL);
CREATE TABLE recall_evidence(
 id TEXT PRIMARY KEY,recording_id TEXT NOT NULL REFERENCES recordings(id) ON DELETE CASCADE,
 segment_id TEXT NOT NULL REFERENCES recording_segments(id) ON DELETE CASCADE,
 source_kind TEXT NOT NULL,source_track_id TEXT NOT NULL,source_id TEXT NOT NULL,
 start_offset_ms INTEGER NOT NULL,end_offset_ms INTEGER NOT NULL,
 started_at_ms INTEGER NOT NULL,ended_at_ms INTEGER NOT NULL,
 revision TEXT NOT NULL,text TEXT NOT NULL,provenance_json TEXT NOT NULL CHECK(json_valid(provenance_json))
);
CREATE INDEX recall_evidence_time ON recall_evidence(started_at_ms,ended_at_ms,id);
CREATE INDEX recall_evidence_recording ON recall_evidence(recording_id,id);
CREATE TABLE recall_source_lineage(evidence_id TEXT PRIMARY KEY,recording_id TEXT NOT NULL);
CREATE INDEX recall_source_lineage_recording ON recall_source_lineage(recording_id,evidence_id);
CREATE TABLE recall_index(evidence_id TEXT PRIMARY KEY REFERENCES recall_evidence(id) ON DELETE CASCADE,source_revision TEXT NOT NULL,embedding_space TEXT NOT NULL,dimensions INTEGER NOT NULL,vector BLOB NOT NULL);
CREATE INDEX recall_index_space ON recall_index(embedding_space,evidence_id);
CREATE VIRTUAL TABLE recall_fts USING fts5(evidence_id UNINDEXED,text,tokenize='unicode61');
CREATE TABLE recall_generations(day_key TEXT PRIMARY KEY,input_fingerprint TEXT NOT NULL,profile_fingerprint TEXT NOT NULL,output_json TEXT NOT NULL CHECK(json_valid(output_json)),status TEXT NOT NULL,updated_at INTEGER NOT NULL);
CREATE TABLE recall_generation_windows(day_key TEXT NOT NULL,window_key TEXT NOT NULL,config_fingerprint TEXT NOT NULL,output_json TEXT NOT NULL CHECK(json_valid(output_json)),lineage_json TEXT NOT NULL CHECK(json_valid(lineage_json)),updated_at INTEGER NOT NULL,PRIMARY KEY(day_key,window_key,config_fingerprint));
CREATE TABLE recall_generation_inputs(day_key TEXT NOT NULL,evidence_id TEXT NOT NULL,revision TEXT NOT NULL,PRIMARY KEY(day_key,evidence_id));
CREATE INDEX recall_generation_source ON recall_generation_inputs(evidence_id,day_key);
CREATE TABLE recall_feedback(day_key TEXT NOT NULL,target_id TEXT NOT NULL,rating TEXT,correction TEXT,PRIMARY KEY(day_key,target_id));
CREATE TABLE recall_tasks(id TEXT PRIMARY KEY,day_key TEXT NOT NULL,action_key TEXT NOT NULL,source_json TEXT NOT NULL CHECK(json_valid(source_json)),title TEXT NOT NULL,edited_title TEXT,notes TEXT NOT NULL DEFAULT '',status TEXT NOT NULL,updated_at INTEGER NOT NULL,UNIQUE(day_key,action_key));
CREATE TABLE recall_day_reviews(day_key TEXT PRIMARY KEY,fingerprint TEXT NOT NULL);
CREATE TABLE recall_session_corrections(day_key TEXT NOT NULL,session_id TEXT NOT NULL,correction TEXT,title_correction TEXT,category_correction TEXT,updated_at INTEGER NOT NULL,PRIMARY KEY(day_key,session_id));
CREATE TABLE recall_jobs(id TEXT PRIMARY KEY,day_key TEXT NOT NULL,kind TEXT NOT NULL,status TEXT NOT NULL,input_fingerprint TEXT NOT NULL,config_fingerprint TEXT NOT NULL,config_json TEXT NOT NULL DEFAULT '{}' CHECK(json_valid(config_json)),manual INTEGER NOT NULL DEFAULT 0,attempt_count INTEGER NOT NULL DEFAULT 0,next_attempt_at_ms INTEGER,progress REAL NOT NULL DEFAULT 0,error_code TEXT,created_at_ms INTEGER NOT NULL,updated_at_ms INTEGER NOT NULL,started_at_ms INTEGER);
CREATE INDEX recall_jobs_status ON recall_jobs(status,next_attempt_at_ms);
CREATE TRIGGER recall_source_deleted AFTER DELETE ON recall_evidence BEGIN
 INSERT INTO recall_dirty_recordings VALUES(OLD.recording_id) ON CONFLICT DO NOTHING;
 UPDATE recall_meta SET revision=revision+1,source_dirty=1 WHERE id=1;
 UPDATE recall_generations SET status='invalidated',output_json='{}' WHERE day_key IN(SELECT day_key FROM recall_generation_inputs WHERE evidence_id=OLD.id);
 DELETE FROM recall_fts WHERE rowid=OLD.rowid;
 DELETE FROM recall_generation_windows WHERE EXISTS(SELECT 1 FROM json_each(lineage_json) input WHERE json_extract(input.value,'$.id')=OLD.id);
END;
CREATE TRIGGER recall_source_revised AFTER UPDATE OF revision ON recall_evidence WHEN OLD.revision!=NEW.revision BEGIN
 INSERT INTO recall_dirty_recordings VALUES(NEW.recording_id) ON CONFLICT DO NOTHING;
 UPDATE recall_meta SET revision=revision+1,source_dirty=1 WHERE id=1;
 UPDATE recall_generations SET status='invalidated',output_json='{}' WHERE day_key IN(SELECT day_key FROM recall_generation_inputs WHERE evidence_id=OLD.id);
 DELETE FROM recall_index WHERE evidence_id=OLD.id;
 DELETE FROM recall_generation_windows WHERE EXISTS(SELECT 1 FROM json_each(lineage_json) input WHERE json_extract(input.value,'$.id')=OLD.id);
END;
CREATE TRIGGER recall_fts_insert AFTER INSERT ON recall_evidence BEGIN
 INSERT INTO recall_fts(rowid,evidence_id,text) VALUES(NEW.rowid,NEW.id,NEW.text);
 INSERT INTO recall_source_lineage(evidence_id,recording_id) VALUES(NEW.id,NEW.recording_id) ON CONFLICT(evidence_id) DO UPDATE SET recording_id=excluded.recording_id;
END;
CREATE TRIGGER recall_fts_update AFTER UPDATE OF text ON recall_evidence BEGIN
 DELETE FROM recall_fts WHERE rowid=OLD.rowid;
 INSERT INTO recall_fts(rowid,evidence_id,text) VALUES(NEW.rowid,NEW.id,NEW.text);
END;
CREATE TRIGGER recall_redaction_insert AFTER INSERT ON redactions BEGIN
 INSERT INTO recall_dirty_recordings VALUES(NEW.recording_id) ON CONFLICT DO NOTHING;
 DELETE FROM recall_evidence WHERE recording_id=NEW.recording_id;
 UPDATE recall_meta SET revision=revision+1,source_dirty=1,privacy_revision=privacy_revision+1 WHERE id=1;
END;
CREATE TRIGGER recall_redaction_delete AFTER DELETE ON redactions BEGIN
 INSERT INTO recall_dirty_recordings VALUES(OLD.recording_id) ON CONFLICT DO NOTHING;
 DELETE FROM recall_evidence WHERE recording_id=OLD.recording_id;
 UPDATE recall_meta SET revision=revision+1,source_dirty=1,privacy_revision=privacy_revision+1 WHERE id=1;
END;
CREATE TRIGGER recall_redaction_update AFTER UPDATE ON redactions BEGIN
 INSERT INTO recall_dirty_recordings VALUES(NEW.recording_id) ON CONFLICT DO NOTHING;
 INSERT INTO recall_dirty_recordings VALUES(OLD.recording_id) ON CONFLICT DO NOTHING;
 DELETE FROM recall_evidence WHERE recording_id IN(OLD.recording_id,NEW.recording_id);
 UPDATE recall_meta SET revision=revision+1,source_dirty=1,privacy_revision=privacy_revision+1 WHERE id=1;
END;
CREATE TRIGGER recall_protection_insert AFTER INSERT ON recording_protection_spans BEGIN
 INSERT INTO recall_dirty_recordings VALUES(NEW.recording_id) ON CONFLICT DO NOTHING;
 DELETE FROM recall_evidence WHERE recording_id=NEW.recording_id;
 UPDATE recall_meta SET revision=revision+1,source_dirty=1,privacy_revision=privacy_revision+1 WHERE id=1;
END;
CREATE TRIGGER recall_protection_update AFTER UPDATE ON recording_protection_spans BEGIN
 INSERT INTO recall_dirty_recordings VALUES(NEW.recording_id) ON CONFLICT DO NOTHING;
 DELETE FROM recall_evidence WHERE recording_id=NEW.recording_id;
 UPDATE recall_meta SET revision=revision+1,source_dirty=1,privacy_revision=privacy_revision+1 WHERE id=1;
END;
CREATE TRIGGER recall_protection_delete AFTER DELETE ON recording_protection_spans BEGIN
 INSERT INTO recall_dirty_recordings VALUES(OLD.recording_id) ON CONFLICT DO NOTHING;
 DELETE FROM recall_evidence WHERE recording_id=OLD.recording_id;
 UPDATE recall_meta SET revision=revision+1,source_dirty=1,privacy_revision=privacy_revision+1 WHERE id=1;
END;
CREATE TRIGGER recall_config_insert AFTER INSERT ON app_settings WHEN NEW.key='ai.recall' OR NEW.key GLOB 'ai.secret.recall.*' BEGIN
 UPDATE recall_meta SET revision=revision+1 WHERE id=1;
END;
CREATE TRIGGER recall_config_update AFTER UPDATE ON app_settings WHEN (NEW.key='ai.recall' OR NEW.key GLOB 'ai.secret.recall.*') AND NEW.value_json!=OLD.value_json BEGIN
 UPDATE recall_meta SET revision=revision+1 WHERE id=1;
END;
CREATE TRIGGER recall_config_delete AFTER DELETE ON app_settings WHEN OLD.key='ai.recall' OR OLD.key GLOB 'ai.secret.recall.*' BEGIN
 UPDATE recall_meta SET revision=revision+1 WHERE id=1;
END;
CREATE TRIGGER recall_recording_delete BEFORE DELETE ON recordings BEGIN
 UPDATE recall_meta SET revision=revision+1,source_dirty=1,privacy_revision=privacy_revision+1 WHERE id=1;
 UPDATE recall_tasks SET title='',source_json='{"inputs":[],"citations":[],"withheld":true}' WHERE EXISTS(
  SELECT 1 FROM json_each(recall_tasks.source_json,'$.inputs') input JOIN recall_source_lineage e ON e.evidence_id=json_extract(input.value,'$.id') WHERE e.recording_id=OLD.id
 );
 UPDATE recall_generations SET status='invalidated',output_json='{}' WHERE day_key IN(SELECT i.day_key FROM recall_generation_inputs i JOIN recall_source_lineage e ON e.evidence_id=i.evidence_id WHERE e.recording_id=OLD.id);
 DELETE FROM recall_generation_inputs WHERE evidence_id IN(SELECT evidence_id FROM recall_source_lineage WHERE recording_id=OLD.id);
 DELETE FROM recall_generation_windows WHERE EXISTS(SELECT 1 FROM json_each(lineage_json) input JOIN recall_source_lineage e ON e.evidence_id=json_extract(input.value,'$.id') WHERE e.recording_id=OLD.id);
 DELETE FROM recall_source_lineage WHERE recording_id=OLD.id;
END;
"#;
