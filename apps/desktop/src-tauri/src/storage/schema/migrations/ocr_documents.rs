use rusqlite_migration::M;

pub(in crate::storage) fn migrations() -> Vec<M<'static>> {
    vec![M::up_with_hook(
        r#"
CREATE TABLE processing_job_ocr_configs (
 job_id TEXT PRIMARY KEY REFERENCES processing_jobs(id) ON DELETE CASCADE,
 config_json TEXT NOT NULL CHECK(json_valid(config_json))
);
CREATE TABLE ocr_document_contents (
 id TEXT PRIMARY KEY NOT NULL,
 recording_id TEXT NOT NULL REFERENCES recordings(id) ON DELETE CASCADE,
 source_track_id TEXT NOT NULL,
 pixel_hash TEXT NOT NULL,
 width INTEGER NOT NULL CHECK(width>=0),height INTEGER NOT NULL CHECK(height>=0),
 config_fingerprint TEXT NOT NULL,
 document_json TEXT NOT NULL CHECK(json_valid(document_json)),
 text TEXT NOT NULL,normalized_text TEXT NOT NULL
);
CREATE INDEX idx_ocr_content_cache ON ocr_document_contents(recording_id,source_track_id,pixel_hash,width,height,config_fingerprint);
CREATE TABLE ocr_frame_occurrences (
 id TEXT PRIMARY KEY NOT NULL,
 recording_id TEXT NOT NULL REFERENCES recordings(id) ON DELETE CASCADE,
 segment_id TEXT NOT NULL REFERENCES recording_segments(id) ON DELETE CASCADE,
 source_track_id TEXT NOT NULL,
 frame_timestamp_ms INTEGER NOT NULL CHECK(frame_timestamp_ms>=0),
 sample_end_ms INTEGER NOT NULL CHECK(sample_end_ms>frame_timestamp_ms),
 source_frame_ref TEXT NOT NULL,width INTEGER NOT NULL CHECK(width>=0),height INTEGER NOT NULL CHECK(height>=0),
 engine TEXT NOT NULL CHECK(engine IN ('tesseract','vision','legacy')),
 provider TEXT NOT NULL,model TEXT,config_fingerprint TEXT NOT NULL,
 status TEXT NOT NULL CHECK(status IN ('succeeded','failed','gap')),
 error_code TEXT,content_id TEXT REFERENCES ocr_document_contents(id),cache_hit INTEGER NOT NULL CHECK(cache_hit IN (0,1)),
 provenance TEXT NOT NULL CHECK(provenance IN ('native','reconstructed')),
 job_id TEXT NOT NULL REFERENCES processing_jobs(id) ON DELETE CASCADE,
 chunk_id TEXT NOT NULL REFERENCES processing_chunks(id) ON DELETE CASCADE,
 attempt_count INTEGER NOT NULL CHECK(attempt_count>=0),
 raw_links_json TEXT NOT NULL DEFAULT '{}' CHECK(json_valid(raw_links_json)),
 CHECK((status='succeeded' AND content_id IS NOT NULL) OR (status!='succeeded' AND content_id IS NULL)),
 UNIQUE(recording_id,segment_id,source_track_id,frame_timestamp_ms)
);
CREATE INDEX idx_ocr_occurrence_timeline ON ocr_frame_occurrences(recording_id,source_track_id,frame_timestamp_ms,id);
CREATE INDEX idx_ocr_occurrence_chunk ON ocr_frame_occurrences(chunk_id);
CREATE TRIGGER prune_unreferenced_ocr_content AFTER DELETE ON ocr_frame_occurrences BEGIN
 DELETE FROM ocr_document_contents WHERE id=OLD.content_id AND NOT EXISTS(SELECT 1 FROM ocr_frame_occurrences WHERE content_id=OLD.content_id);
END;
CREATE TRIGGER prune_replaced_ocr_content AFTER UPDATE OF content_id ON ocr_frame_occurrences BEGIN
 DELETE FROM ocr_document_contents WHERE id=OLD.content_id AND NOT EXISTS(SELECT 1 FROM ocr_frame_occurrences WHERE content_id=OLD.content_id);
END;
"#,
        |transaction| {
            crate::storage::ai::ocr_documents::reconstruct_legacy_documents(transaction)?;
            Ok(())
        },
    )]
}
