use rusqlite_migration::M;

pub(super) fn migrations() -> Vec<M<'static>> {
    vec![M::up(
        r#"
CREATE TABLE audio_cleanup_generations (
 id TEXT PRIMARY KEY,
 recording_id TEXT NOT NULL REFERENCES recordings(id) ON DELETE CASCADE,
 config_json TEXT NOT NULL CHECK(json_valid(config_json)),
 source_fingerprint TEXT NOT NULL,
 status TEXT NOT NULL CHECK(status IN ('running','ready','cancelled','failed','invalidated')),
 completed_ms INTEGER NOT NULL DEFAULT 0,
 matched_ms INTEGER NOT NULL DEFAULT 0,
 manifest_json TEXT NOT NULL DEFAULT '[]' CHECK(json_valid(manifest_json)),
 error TEXT,
 created_at_ms INTEGER NOT NULL
);
CREATE INDEX audio_cleanup_recording ON audio_cleanup_generations(recording_id,created_at_ms);
CREATE TABLE recording_audio_cleanup (
 recording_id TEXT PRIMARY KEY REFERENCES recordings(id) ON DELETE CASCADE,
 config_json TEXT NOT NULL DEFAULT '{"mode":"off","matchThreshold":0.65,"amount":1.0}' CHECK(json_valid(config_json)),
 active_revision TEXT REFERENCES audio_cleanup_generations(id),
 pending_revision TEXT REFERENCES audio_cleanup_generations(id)
);
CREATE TABLE processing_audio_cleanup_inputs (
 job_id TEXT PRIMARY KEY REFERENCES processing_jobs(id) ON DELETE CASCADE,
 revision TEXT REFERENCES audio_cleanup_generations(id)
);
-- A microphone result made from a reference depends on reference eligibility too.
CREATE TRIGGER cleanup_redaction_insert AFTER INSERT ON redactions BEGIN
 UPDATE audio_cleanup_generations SET status='invalidated' WHERE recording_id=NEW.recording_id;
 DELETE FROM transcript_segments WHERE source_role='microphone' AND job_id IN
  (SELECT job_id FROM processing_audio_cleanup_inputs WHERE revision IS NOT NULL AND job_id IN
   (SELECT id FROM processing_jobs WHERE recording_id=NEW.recording_id));
END;
CREATE TRIGGER cleanup_redaction_update AFTER UPDATE ON redactions BEGIN
 UPDATE audio_cleanup_generations SET status='invalidated' WHERE recording_id=NEW.recording_id;
 DELETE FROM transcript_segments WHERE source_role='microphone' AND job_id IN
  (SELECT job_id FROM processing_audio_cleanup_inputs WHERE revision IS NOT NULL AND job_id IN
   (SELECT id FROM processing_jobs WHERE recording_id=NEW.recording_id));
END;
CREATE TRIGGER cleanup_redaction_delete AFTER DELETE ON redactions BEGIN
 UPDATE audio_cleanup_generations SET status='invalidated' WHERE recording_id=OLD.recording_id;
END;
"#,
    )]
}
