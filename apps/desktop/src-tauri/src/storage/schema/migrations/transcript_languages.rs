use rusqlite_migration::M;

pub(in crate::storage) fn migrations() -> Vec<M<'static>> {
    vec![M::up(
        r#"
CREATE TABLE transcript_language_context (
 transcript_segment_id TEXT PRIMARY KEY REFERENCES transcript_segments(id) ON DELETE CASCADE,
 detection_segment_id TEXT NOT NULL REFERENCES transcript_segments(id) ON DELETE CASCADE,
 language TEXT NOT NULL,
 confidence REAL NOT NULL CHECK(confidence>=0.6 AND confidence<=1),
 config_fingerprint TEXT NOT NULL,
 detection_start_ms INTEGER NOT NULL,
 detection_end_ms INTEGER NOT NULL CHECK(detection_end_ms>detection_start_ms)
);
CREATE INDEX idx_transcript_language_detection ON transcript_language_context(detection_segment_id);
CREATE INDEX idx_transcript_language_predecessor ON transcript_segments(recording_id,track_id,end_offset_ms);
"#,
    )]
}
