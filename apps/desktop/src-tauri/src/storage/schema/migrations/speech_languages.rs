use rusqlite_migration::M;

pub(in crate::storage) fn migrations() -> Vec<M<'static>> {
    vec![M::up_with_hook(
        "CREATE TABLE processing_speech_languages (
          unit_id TEXT PRIMARY KEY REFERENCES processing_speech_units(id) ON DELETE CASCADE,
          decision_json TEXT NOT NULL CHECK(json_valid(decision_json))
        );
        CREATE TABLE processing_speech_language_support (
          unit_id TEXT NOT NULL REFERENCES processing_speech_units(id) ON DELETE CASCADE,
          source_unit_id TEXT NOT NULL REFERENCES processing_speech_units(id) ON DELETE CASCADE,
          recording_id TEXT NOT NULL REFERENCES recordings(id) ON DELETE CASCADE,
          track_ids_json TEXT NOT NULL CHECK(json_valid(track_ids_json)),
          start_ms INTEGER NOT NULL,
          end_ms INTEGER NOT NULL CHECK(end_ms>start_ms),
          PRIMARY KEY(unit_id,source_unit_id,start_ms)
        );
        CREATE INDEX idx_speech_language_support_source ON processing_speech_language_support(source_unit_id,unit_id);
        CREATE TRIGGER speech_language_source_delete BEFORE DELETE ON processing_speech_units BEGIN
          DELETE FROM transcript_segments WHERE id IN(SELECT t.transcript_id FROM processing_speech_unit_transcripts t JOIN processing_speech_language_support s ON s.unit_id=t.unit_id WHERE s.source_unit_id=OLD.id);
          DELETE FROM processing_speech_languages WHERE unit_id IN(SELECT unit_id FROM processing_speech_language_support WHERE source_unit_id=OLD.id);
        END;",
        |connection| {
            let sql: String = connection.query_row("SELECT sql FROM sqlite_master WHERE type='view' AND name='published_transcript_segments'", [], |row| row.get(0))?;
            connection.execute_batch("DROP VIEW published_transcript_segments;")?;
            connection.execute_batch(&sql.replace("CREATE VIEW published_transcript_segments AS", "CREATE VIEW published_transcript_segments_without_language AS"))?;
            connection.execute_batch("CREATE VIEW published_transcript_segments AS
              SELECT t.* FROM published_transcript_segments_without_language t
              WHERE NOT EXISTS(SELECT 1 FROM processing_speech_unit_transcripts l
                JOIN processing_speech_language_support s ON s.unit_id=l.unit_id
                JOIN recordings recording ON recording.id=s.recording_id
                WHERE l.transcript_id=t.id AND (
                  EXISTS(SELECT 1 FROM redactions r WHERE r.recording_id=s.recording_id AND r.start_offset_ms<s.end_ms AND r.end_offset_ms>s.start_ms
                    AND (json_array_length(r.track_ids_json)=0 OR EXISTS(SELECT 1 FROM json_each(r.track_ids_json) target JOIN json_each(s.track_ids_json) source ON source.value=target.value)))
                  OR EXISTS(SELECT 1 FROM recording_protection_spans p WHERE p.recording_id=s.recording_id AND p.status!='materialized'
                    AND (recording.status NOT IN('recording','paused') OR (p.start_offset_ms<s.end_ms AND (p.end_offset_ms IS NULL OR p.end_offset_ms>s.start_ms))))
                  OR (recording.status IN('recording','paused') AND NOT EXISTS(SELECT 1 FROM recording_live_state live WHERE live.recording_id=s.recording_id AND live.classified_through_ms>=s.end_ms))
                ));")?;
            Ok(())
        },
    )]
}
