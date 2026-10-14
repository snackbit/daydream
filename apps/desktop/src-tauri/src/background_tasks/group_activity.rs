//! Bounded operational metadata. Captured content is never part of this projection.
use super::{ProcessingLaneActivity, RecordingProcessingLane};
use crate::storage;
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::HashMap;

pub(super) fn read(
    connection: &Connection,
    lane: &RecordingProcessingLane,
) -> rusqlite::Result<Vec<ProcessingLaneActivity>> {
    let jobs = serde_json::to_string(&lane.job_ids).map_err(|_| rusqlite::Error::InvalidQuery)?;
    let mut query = connection.prepare(r#"
WITH work AS (
 SELECT c.*,ROW_NUMBER() OVER(PARTITION BY c.status ORDER BY c.start_offset_ms,c.id) position
 FROM processing_chunks c WHERE c.job_id IN(SELECT value FROM json_each(?1)) AND c.source_kind=?2 AND c.status IN('running','queued')
)
SELECT c.id,c.job_id,c.track_id,t.label,c.start_offset_ms,c.end_offset_ms,c.status,
 COALESCE(r.phase,'queued'),r.started_at_ms,r.phase_started_at_ms,r.last_progress_at_ms,r.next_retry_at_ms,j.status,
 CASE WHEN c.source_kind IN('ocr','vlm') THEN r.total_units END,
 (SELECT COUNT(*) FROM visual_frame_processing_states v WHERE v.chunk_id=c.id AND v.status IN('succeeded','failed'))
FROM work c LEFT JOIN processing_chunk_runtime r ON r.chunk_id=c.id LEFT JOIN recording_tracks t ON t.id=c.track_id
JOIN processing_jobs j ON j.id=c.job_id WHERE position<=3 ORDER BY c.status='running' DESC,c.start_offset_ms,c.id
"#)?;
    let rows = query
        .query_map(params![jobs, lane.kind], |row| {
            Ok((
                ProcessingLaneActivity {
                    chunk_id: row.get(0)?,
                    job_id: row.get(1)?,
                    source_track_id: row.get(2)?,
                    source_label: row.get(3)?,
                    start_offset_ms: row.get(4)?,
                    end_offset_ms: row.get(5)?,
                    status: row.get(6)?,
                    phase: row.get(7)?,
                    started_at_ms: row.get(8)?,
                    phase_started_at_ms: row.get(9)?,
                    last_progress_at_ms: row.get(10)?,
                    next_retry_at_ms: row.get(11)?,
                    waiting_reason: None,
                    worker_active: None,
                    total_units: row.get(13)?,
                    completed_units: if row.get::<_, Option<i64>>(13)?.is_some() {
                        Some(row.get(14)?)
                    } else {
                        None
                    },
                },
                row.get::<_, String>(12)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut peers = HashMap::new();
    let mut result = Vec::new();
    for (mut activity, parent) in rows {
        if !peers.contains_key(&activity.job_id) {
            let chunks = storage::list_processing_chunks_by_status(
                connection,
                &activity.job_id,
                &["queued", "running"],
                None,
            )?;
            let chronological = connection.query_row("SELECT visual_protocol='changes-v1' FROM processing_job_runtime WHERE job_id=?1", [&activity.job_id], |row| row.get::<_,bool>(0)).optional()?.unwrap_or(false);
            peers.insert(activity.job_id.clone(), (chunks, chronological));
        }
        let (chunks, chronological) = &peers[&activity.job_id];
        let chunk = chunks
            .iter()
            .find(|chunk| chunk.id == activity.chunk_id)
            .ok_or(rusqlite::Error::QueryReturnedNoRows)?;
        let protection = crate::recording_privacy::range_protection_ready(
            connection,
            &chunk.recording_id,
            chunk.start_offset_ms.unwrap_or(0),
            chunk.end_offset_ms.unwrap_or(i64::MAX),
        )?;
        let visible = chunk.start_offset_ms.is_some()
            && chunk.end_offset_ms.is_some()
            && crate::recording_privacy::chunk_eligible(connection, chunk)?;
        if !visible {
            activity.source_track_id = None;
            activity.source_label = None;
            activity.start_offset_ms = None;
            activity.end_offset_ms = None;
        }
        activity.waiting_reason = if parent == "paused" {
            Some("paused")
        } else if !protection {
            Some("protection")
        } else if activity.phase == "waiting-for-audio-context" {
            Some("live-input")
        } else if activity
            .next_retry_at_ms
            .is_some_and(|time| time > super::now_ms())
        {
            Some("retry")
        } else if activity.status == "queued" {
            crate::ai::processing::chunk_dependency_wait_reason(chunk, chunks, *chronological)
                .or(Some("unknown"))
        } else {
            None
        }
        .map(str::to_owned);
        result.push(activity);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn finding_audio_activity_does_not_expose_frame_counters() {
        let storage = fixture();
        let settings = crate::settings::load_app_settings(&storage).unwrap();
        storage
            .with_connection(|connection| {
                recording(connection, "audio-progress");
                for kind in ["audio", "ocr", "vlm"] {
                    job(connection, "audio-progress", kind, 10, kind, "running");
                    storage::set_processing_chunk_total(
                        connection,
                        &fixture_id(&format!("{kind}-chunk")),
                        6,
                    )?;
                    let lane =
                        groups::lane(connection, &fixture_id("audio-progress"), kind, &settings)?;
                    assert_eq!(
                        lane.activity[0].total_units,
                        if kind == "audio" { None } else { Some(6) }
                    );
                    assert_eq!(
                        lane.activity[0].completed_units,
                        if kind == "audio" { None } else { Some(0) }
                    );
                }
                Ok(())
            })
            .unwrap();
    }
    #[test]
    fn retry_progress_counts_retained_successes_and_the_retried_checkpoint() {
        use super::super::{group_test_fixtures::*, groups};
        let storage = fixture();
        let settings = crate::settings::load_app_settings(&storage).unwrap();
        storage.with_connection(|connection| {
            recording(connection, "retry-progress");
            job(connection, "retry-progress", "visual", 10, "vlm", "running");
            let chunk = fixture_id("visual-chunk");
            for index in 0..6 {
                connection.execute("INSERT INTO visual_frame_processing_states(id,chunk_id,job_id,recording_id,source_kind,frame_timestamp_ms,source_frame_ref,region_x,region_y,region_width,region_height,status,progress,output_count,created_at_ms,updated_at_ms) VALUES(?1,?2,?3,?4,'vlm',?5,?8,0,0,64,64,?6,?7,0,1,1)",params![fixture_id(&format!("checkpoint-{index}")),chunk,fixture_id("visual"),fixture_id("retry-progress"),index*100,if index<5 {"succeeded"}else{"queued"},if index<5 {1.}else{0.},format!("synthetic-checkpoint-{index}")])?;
            }
            storage::set_processing_chunk_visual_total(connection, &chunk)?;
            let lane = groups::lane(connection, &fixture_id("retry-progress"), "vlm", &settings)?;
            assert_eq!(lane.activity[0].total_units, Some(6));
            assert_eq!(lane.activity[0].completed_units, Some(5));
            connection.execute("UPDATE visual_frame_processing_states SET status='succeeded',progress=1 WHERE id=?1",[fixture_id("checkpoint-5")])?;
            let lane = groups::lane(connection, &fixture_id("retry-progress"), "vlm", &settings)?;
            assert_eq!(lane.activity[0].total_units, Some(6));
            assert_eq!(lane.activity[0].completed_units, Some(6));
            Ok(())
        }).unwrap();
    }
    use crate::background_tasks::{group_test_fixtures::*, groups};

    #[test]
    fn legacy_work_without_source_offsets_remains_visible_without_exposing_unverified_source_metadata(
    ) {
        let storage = fixture();
        let settings = crate::settings::load_app_settings(&storage).unwrap();
        storage
            .with_connection(|connection| {
                recording(connection, "a");
                job(connection, "a", "ocr", 10, "ocr", "queued");
                connection.execute(
                    "UPDATE processing_chunks SET start_offset_ms=NULL,end_offset_ms=NULL",
                    [],
                )?;
                let lane = groups::lane(connection, &fixture_id("a"), "ocr", &settings)?;
                assert_eq!(lane.queued, 1);
                assert_eq!(lane.activity.len(), 1);
                assert!(lane.activity[0].start_offset_ms.is_none());
                assert!(lane.activity[0].source_label.is_none());
                Ok(())
            })
            .unwrap();
    }

    fn visual_source(connection: &Connection) {
        storage::upsert_recording_track(
            connection,
            &storage::UpsertRecordingTrackInput {
                id: fixture_id("screen"),
                recording_id: fixture_id("a"),
                kind: "screen".into(),
                label: "Synthetic monitor".into(),
                source_id: None,
                path: None,
                codec: None,
                created_at_ms: 1,
            },
        )
        .unwrap();
        let segment = storage::create_recording_segment(
            connection,
            &storage::CreateRecordingSegmentInput {
                id: fixture_id("segment"),
                recording_id: fixture_id("a"),
                file_path: "/tmp/synthetic-overview.mkv".into(),
                segment_index: 1,
                started_at_ms: 1,
            },
        )
        .unwrap();
        storage::finalize_recording_segment(
            connection,
            &storage::FinalizeRecordingSegmentInput {
                id: segment.id,
                duration_ms: 2000,
                ended_at_ms: 2001,
                updated_at_ms: 2001,
                video_stream_index: Some(0),
                desktop_audio_stream_index: None,
                microphone_audio_stream_index: None,
                probe_json: None,
            },
        )
        .unwrap();
        connection
            .execute(
                "UPDATE processing_chunks SET track_id=?1,segment_id=?2 WHERE recording_id=?3",
                params![fixture_id("screen"), fixture_id("segment"), fixture_id("a")],
            )
            .unwrap();
    }
    #[test]
    fn overview_explains_ocr_dependencies_retry_delay_and_pause_and_masks_redacted_sources() {
        let storage = fixture();
        let settings = crate::settings::load_app_settings(&storage).unwrap();
        storage.with_connection(|connection| {
            recording(connection,"a"); job(connection,"a","ocr",10,"ocr","running");job(connection,"a","vlm",20,"vlm","queued");visual_source(connection);
            // Dependencies belong to one processing job and source interval.
            connection.execute("UPDATE processing_chunks SET job_id=?1 WHERE id=?2",params![fixture_id("ocr"),fixture_id("vlm-chunk")])?;
            let lane=groups::lane(connection,&fixture_id("a"),"vlm",&settings)?;
            let work=&lane.activity[0];
            assert_eq!(work.waiting_reason.as_deref(),Some("ocr"));
            assert_eq!(work.source_label.as_deref(),Some("Synthetic monitor"));
            assert_eq!(work.start_offset_ms,Some(0));
            connection.execute("INSERT INTO processing_chunk_runtime(chunk_id,next_retry_at_ms,phase) VALUES(?1,?2,'retrying')",params![fixture_id("vlm-chunk"),crate::background_tasks::now_ms()+10_000])?;
            assert_eq!(groups::lane(connection,&fixture_id("a"),"vlm",&settings)?.activity[0].waiting_reason.as_deref(),Some("retry"));
            storage::update_processing_job_status(connection,&fixture_id("ocr"),"paused",None,30)?;
            assert_eq!(groups::lane(connection,&fixture_id("a"),"vlm",&settings)?.activity[0].waiting_reason.as_deref(),Some("paused"));
            storage::create_redaction(connection,&storage::CreateRedactionInput {id:fixture_id("cut"),recording_id:fixture_id("a"),start_offset_ms:0,end_offset_ms:500,track_ids_json:serde_json::to_string(&[fixture_id("screen")]).unwrap(),reason:None,created_at_ms:40})?;
            let hidden=groups::lane(connection,&fixture_id("a"),"vlm",&settings)?;
            assert!(hidden.activity[0].source_label.is_none());assert!(hidden.activity[0].start_offset_ms.is_none());assert!(hidden.activity[0].source_track_id.is_none());
            Ok(())
        }).unwrap();
    }
    #[test]
    fn activity_is_bounded_and_progress_time_changes_only_for_measured_progress() {
        let storage = fixture();
        let settings = crate::settings::load_app_settings(&storage).unwrap();
        storage
            .with_connection(|connection| {
                recording(connection, "a");
                job(connection, "a", "ocr", 10, "ocr", "queued");
                visual_source(connection);
                let id = fixture_id("ocr-chunk");
                storage::claim_processing_chunk(connection, &id, 0.05, 20)?;
                storage::set_processing_chunk_phase(connection, &id, "recognizing-text")?;
                let before = groups::lane(connection, &fixture_id("a"), "ocr", &settings)?;
                assert_eq!(before.activity[0].started_at_ms, Some(20));
                assert!(before.activity[0].phase_started_at_ms.is_some());
                assert!(before.activity[0].last_progress_at_ms.is_none());
                storage::update_processing_chunk_progress(connection, &id, 0.5, 30)?;
                storage::update_processing_chunk_progress(connection, &id, 0.5, 40)?;
                let measured = groups::lane(connection, &fixture_id("a"), "ocr", &settings)?;
                assert_eq!(measured.activity[0].last_progress_at_ms, Some(30));
                for index in 0..10 {
                    let chunk = storage::get_processing_chunk(connection, &id)?.unwrap();
                    storage::create_processing_chunk(
                        connection,
                        &storage::CreateProcessingChunkInput {
                            id: fixture_id(&format!("extra-{index}")),
                            job_id: chunk.job_id,
                            recording_id: chunk.recording_id,
                            source_kind: "ocr".into(),
                            track_id: chunk.track_id,
                            segment_id: chunk.segment_id,
                            start_offset_ms: Some(1000),
                            end_offset_ms: Some(2000),
                            status: "queued".into(),
                            progress: 0.,
                            attempt_count: 0,
                            error_code: None,
                            result_code: None,
                            created_at_ms: 10,
                            updated_at_ms: 10,
                        },
                    )?;
                }
                let bounded = groups::lane(connection, &fixture_id("a"), "ocr", &settings)?;
                assert_eq!(bounded.queued, 10);
                assert_eq!(bounded.activity.len(), 4);
                Ok(())
            })
            .unwrap();
    }
}
