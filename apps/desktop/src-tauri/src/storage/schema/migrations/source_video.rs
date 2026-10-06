use rusqlite_migration::M;

pub(in crate::storage) fn migrations() -> Vec<M<'static>> {
    vec![M::up(
        r#"
ALTER TABLE recorder_scenes ADD COLUMN independent_video INTEGER NOT NULL DEFAULT 0 CHECK(independent_video IN(0,1));
CREATE TABLE recording_video_sources (
 track_id TEXT PRIMARY KEY REFERENCES recording_tracks(id) ON DELETE CASCADE,
 recording_id TEXT NOT NULL REFERENCES recordings(id) ON DELETE CASCADE,
 source_id TEXT NOT NULL,label TEXT NOT NULL,
 width INTEGER NOT NULL CHECK(width>0),height INTEGER NOT NULL CHECK(height>0),
 config_json TEXT NOT NULL CHECK(json_valid(config_json)),
 UNIQUE(recording_id,source_id)
);
CREATE TABLE recording_segment_videos (
 segment_id TEXT NOT NULL REFERENCES recording_segments(id) ON DELETE CASCADE,
 track_id TEXT NOT NULL REFERENCES recording_tracks(id) ON DELETE CASCADE,
 stream_index INTEGER NOT NULL CHECK(stream_index>=0),
 width INTEGER NOT NULL CHECK(width>0),height INTEGER NOT NULL CHECK(height>0),
 fps REAL NOT NULL CHECK(fps>0),
 PRIMARY KEY(segment_id,track_id),UNIQUE(segment_id,stream_index)
);
CREATE INDEX idx_recording_video_sources_recording ON recording_video_sources(recording_id);
"#,
    )]
}
