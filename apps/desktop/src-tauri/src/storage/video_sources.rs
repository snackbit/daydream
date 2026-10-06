//! Stable video identity and per-fragment mux mappings, with legacy read support.
use super::*;
use rusqlite::{params, Connection};

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingVideoSource {
    pub track_id: String,
    pub source_id: String,
    pub label: String,
    pub width: u32,
    pub height: u32,
    pub config_json: String,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SegmentVideoStream {
    pub track_id: String,
    pub source_id: Option<String>,
    pub label: String,
    pub stream_index: i64,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub fps: Option<f64>,
}

pub fn save_recording_video_sources(
    connection: &Connection,
    recording_id: &str,
    sources: &[RecordingVideoSource],
    timestamp: i64,
) -> rusqlite::Result<()> {
    for source in sources {
        connection.execute("INSERT INTO recording_tracks(id,recording_id,kind,label,source_id,path,codec,created_at_ms) VALUES(?1,?2,'screen',?3,?4,NULL,'h264',?5) ON CONFLICT(id) DO NOTHING",params![source.track_id,recording_id,source.label,source.source_id,timestamp])?;
        connection.execute("INSERT INTO recording_video_sources(track_id,recording_id,source_id,label,width,height,config_json) VALUES(?1,?2,?3,?4,?5,?6,?7)",params![source.track_id,recording_id,source.source_id,source.label,source.width,source.height,source.config_json])?;
    }
    Ok(())
}

pub fn save_segment_video_streams(
    connection: &Connection,
    segment_id: &str,
    sources: &[RecordingVideoSource],
    fps: f64,
) -> rusqlite::Result<()> {
    for (index, source) in sources.iter().enumerate() {
        connection.execute("INSERT INTO recording_segment_videos(segment_id,track_id,stream_index,width,height,fps) VALUES(?1,?2,?3,?4,?5,?6) ON CONFLICT(segment_id,track_id) DO UPDATE SET stream_index=excluded.stream_index,width=excluded.width,height=excluded.height,fps=excluded.fps",params![segment_id,source.track_id,index as i64,source.width,source.height,fps])?;
    }
    Ok(())
}

pub fn segment_video_streams(
    connection: &Connection,
    segment: &RecordingSegmentRow,
) -> rusqlite::Result<Vec<SegmentVideoStream>> {
    let mut statement=connection.prepare("SELECT v.track_id,s.source_id,s.label,v.stream_index,v.width,v.height,v.fps FROM recording_segment_videos v JOIN recording_video_sources s ON s.track_id=v.track_id WHERE v.segment_id=?1 ORDER BY v.stream_index")?;
    let rows = statement.query_map([&segment.id], |row| {
        Ok(SegmentVideoStream {
            track_id: row.get(0)?,
            source_id: row.get(1)?,
            label: row.get(2)?,
            stream_index: row.get(3)?,
            width: row.get(4)?,
            height: row.get(5)?,
            fps: row.get(6)?,
        })
    })?;
    let mut streams = rows.collect::<rusqlite::Result<Vec<_>>>()?;
    if streams.is_empty() {
        if let Some(index) = segment.video_stream_index {
            streams.push(SegmentVideoStream {
                track_id: format!("video-{index}"),
                source_id: None,
                label: "Video".into(),
                stream_index: index,
                width: None,
                height: None,
                fps: None,
            });
        }
    }
    Ok(streams)
}

pub fn recording_video_streams(
    connection: &Connection,
    recording_id: &str,
) -> rusqlite::Result<std::collections::BTreeMap<String, Vec<SegmentVideoStream>>> {
    list_recording_segments(connection, recording_id)?
        .into_iter()
        .map(|segment| {
            Ok((
                segment.id.clone(),
                segment_video_streams(connection, &segment)?,
            ))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_identity_maps_each_fragment_and_cascades_on_recording_deletion() {
        let storage =
            super::super::test_storage(std::path::PathBuf::from("/tmp/synthetic-video-metadata"));
        storage
            .with_connection(|connection| {
                let recording = uuid::Uuid::now_v7().to_string();
                create_recording(
                    connection,
                    &CreateRecordingInput {
                        id: recording.clone(),
                        active_scene_id: None,
                        created_at_ms: 1,
                        default_fps: 1.0,
                        notes: None,
                        output_root: None,
                        quality_label: "balanced-ocr".into(),
                        storage_dir: "/tmp/synthetic-video-metadata".into(),
                    },
                )?;
                let sources = vec![
                    RecordingVideoSource {
                        track_id: uuid::Uuid::now_v7().to_string(),
                        source_id: "monitor-a".into(),
                        label: "Synthetic A".into(),
                        width: 2560,
                        height: 720,
                        config_json: "{}".into(),
                    },
                    RecordingVideoSource {
                        track_id: uuid::Uuid::now_v7().to_string(),
                        source_id: "monitor-b".into(),
                        label: "Synthetic B".into(),
                        width: 2560,
                        height: 1440,
                        config_json: "{}".into(),
                    },
                ];
                save_recording_video_sources(connection, &recording, &sources, 1)?;
                for index in 1..=2 {
                    let segment = create_recording_segment(
                        connection,
                        &CreateRecordingSegmentInput {
                            id: uuid::Uuid::now_v7().to_string(),
                            recording_id: recording.clone(),
                            segment_index: index,
                            file_path: format!("/tmp/synthetic-{index}.mkv"),
                            started_at_ms: 1,
                        },
                    )?;
                    save_segment_video_streams(connection, &segment.id, &sources, 1.0)?;
                    let streams = segment_video_streams(connection, &segment)?;
                    assert_eq!(streams.len(), 2);
                    assert_eq!(streams[0].track_id, sources[0].track_id);
                    assert_eq!(streams[1].stream_index, 1);
                    assert_eq!(streams[0].width, Some(2560));
                }
                update_recording_status(connection, &recording, "failed")?;
                delete_recording(connection, &recording)?;
                assert_eq!(
                    connection.query_row(
                        "SELECT COUNT(*) FROM recording_segment_videos",
                        [],
                        |row| row.get::<_, i64>(0)
                    )?,
                    0
                );
                assert_eq!(
                    connection.query_row(
                        "SELECT COUNT(*) FROM recording_video_sources",
                        [],
                        |row| row.get::<_, i64>(0)
                    )?,
                    0
                );
                Ok(())
            })
            .unwrap();
    }
}
