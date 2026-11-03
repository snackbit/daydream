use rusqlite_migration::M;

pub(in crate::storage) fn migrations() -> Vec<M<'static>> {
    vec![M::up(
        r#"
CREATE TABLE studio_media_queue (
 recording_id TEXT PRIMARY KEY REFERENCES recordings(id) ON DELETE CASCADE,
 generation INTEGER NOT NULL DEFAULT 1,
 priority INTEGER NOT NULL DEFAULT 0,
 status TEXT NOT NULL DEFAULT 'queued' CHECK(status IN('queued','running','ready','failed')),
 error_code TEXT,
 updated_at_ms INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX studio_media_pending ON studio_media_queue(status,priority DESC,updated_at_ms);
CREATE TRIGGER studio_media_classified AFTER UPDATE OF classified_through_ms ON recording_live_state
WHEN NEW.classified_through_ms>OLD.classified_through_ms AND EXISTS(
 SELECT 1 FROM recording_segments s WHERE s.recording_id=NEW.recording_id AND s.state='finalized'
 AND s.timeline_base_ms+s.duration_ms>OLD.classified_through_ms AND s.timeline_base_ms+s.duration_ms<=NEW.classified_through_ms
) BEGIN
 INSERT INTO studio_media_queue(recording_id,priority) VALUES(NEW.recording_id,1)
 ON CONFLICT(recording_id) DO UPDATE SET generation=generation+1,status='queued',priority=MAX(priority,1),error_code=NULL;
END;
INSERT INTO studio_media_queue(recording_id) SELECT id FROM recordings;
CREATE TRIGGER studio_media_segment_finalized AFTER UPDATE OF state ON recording_segments
WHEN NEW.state='finalized' AND OLD.state!='finalized' BEGIN
 INSERT INTO studio_media_queue(recording_id,priority) VALUES(NEW.recording_id,1)
 ON CONFLICT(recording_id) DO UPDATE SET generation=generation+1,status='queued',priority=MAX(priority,1),error_code=NULL;
END;
CREATE TRIGGER studio_media_recording_completed AFTER UPDATE OF status ON recordings
WHEN NEW.status IN('completed','recovered-partial') AND OLD.status!=NEW.status BEGIN
 INSERT INTO studio_media_queue(recording_id,priority) VALUES(NEW.id,1)
 ON CONFLICT(recording_id) DO UPDATE SET generation=generation+1,status='queued',priority=MAX(priority,1),error_code=NULL;
END;
CREATE TABLE studio_media_assets (
 recording_id TEXT NOT NULL REFERENCES recordings(id) ON DELETE CASCADE,
 asset_key TEXT NOT NULL,
 source_fingerprint TEXT NOT NULL,
 path TEXT NOT NULL,
 size_bytes INTEGER NOT NULL CHECK(size_bytes>=0),
 PRIMARY KEY(recording_id,asset_key)
);
CREATE TABLE waveform_cache_entries (
 source_key TEXT PRIMARY KEY,
 recording_id TEXT NOT NULL REFERENCES recordings(id) ON DELETE CASCADE,
 source_kind TEXT NOT NULL,
 maximum REAL NOT NULL CHECK(maximum>=0)
);
CREATE TABLE recording_disk_usage (
 recording_id TEXT PRIMARY KEY REFERENCES recordings(id) ON DELETE CASCADE,
 size_bytes INTEGER NOT NULL CHECK(size_bytes>=0),
 measured_at_ms INTEGER NOT NULL
);
CREATE TRIGGER waveform_catalog_redaction_insert AFTER INSERT ON redactions BEGIN
 DELETE FROM waveform_cache_entries WHERE recording_id=NEW.recording_id;
END;
CREATE TRIGGER waveform_catalog_redaction_update AFTER UPDATE ON redactions BEGIN
 DELETE FROM waveform_cache_entries WHERE recording_id=NEW.recording_id;
END;
CREATE TRIGGER waveform_catalog_redaction_delete AFTER DELETE ON redactions BEGIN
 DELETE FROM waveform_cache_entries WHERE recording_id=OLD.recording_id;
END;
"#,
    )]
}
