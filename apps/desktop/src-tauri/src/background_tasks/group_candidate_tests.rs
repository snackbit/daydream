use super::super::{group_test_fixtures::*, groups::list, *};
use crate::storage;

#[test]
fn large_history_counts_and_filters_do_not_project_or_probe_unrequested_recordings() {
    let storage = fixture();
    storage
        .with_connection(|connection| {
            for index in 0..200 {
                let label = format!("history-{index:03}");
                recording(connection, &label);
                job(
                    connection,
                    &label,
                    &format!("old-{index:03}"),
                    1,
                    "ocr",
                    "failed",
                );
                job(
                    connection,
                    &label,
                    &format!("current-{index:03}"),
                    1000 + index,
                    "ocr",
                    "succeeded",
                );
                if index % 10 == 0 {
                    job(
                        connection,
                        &label,
                        &format!("active-{index:03}"),
                        2000 + index,
                        "audio",
                        "queued",
                    );
                }
                let segment = storage::create_recording_segment(
                    connection,
                    &storage::CreateRecordingSegmentInput {
                        id: fixture_id(&format!("segment-{index:03}")),
                        recording_id: fixture_id(&label),
                        file_path: format!("/tmp/daydream-synthetic-unavailable-{index:03}.mkv"),
                        segment_index: 1,
                        started_at_ms: 1,
                    },
                )?;
                storage::finalize_recording_segment(
                    connection,
                    &storage::FinalizeRecordingSegmentInput {
                        id: segment.id,
                        desktop_audio_stream_index: Some(1),
                        microphone_audio_stream_index: None,
                        video_stream_index: Some(0),
                        duration_ms: 1000,
                        ended_at_ms: 1001,
                        probe_json: None,
                        updated_at_ms: 1001,
                    },
                )?;
            }
            Ok(())
        })
        .unwrap();
    let page = list(
        ListRecordingProcessingGroupsInput {
            limit: Some(3),
            ..Default::default()
        },
        &storage,
    )
    .unwrap();
    assert_eq!(page.groups.len(), 3);
    assert!(page.next_cursor.is_some());
    assert_eq!(page.counts.all, 200);
    assert_eq!(page.counts.active, 20);
    assert_eq!(page.counts.history, 180);
    assert_eq!(page.counts.attention, 0);
    assert!(page.groups.iter().all(|group| group.lanes[1].total == 1
        && group.lanes[1].failed == 0
        && group.lanes.len() == 4));
    storage
        .with_connection(|connection| {
            let probes: i64 =
                connection.query_row("SELECT COUNT(*) FROM recall_source_files", [], |row| {
                    row.get(0)
                })?;
            assert_eq!(
                probes, 3,
                "only requested recordings reach the actual source-file availability owner"
            );
            let pending: i64 = connection.query_row(
                "SELECT COUNT(*) FROM recall_dirty_recordings",
                [],
                |row| row.get(0),
            )?;
            assert_eq!(pending, 197);
            Ok(())
        })
        .unwrap();
    let history = list(
        ListRecordingProcessingGroupsInput {
            filter: Some("history".into()),
            limit: Some(2),
            ..Default::default()
        },
        &storage,
    )
    .unwrap();
    assert_eq!(history.groups.len(), 2);
    assert!(history
        .groups
        .iter()
        .all(|group| group.status == "succeeded"));
    let known = page
        .groups
        .iter()
        .map(|group| group.recording_id.clone())
        .collect();
    let empty = list(
        ListRecordingProcessingGroupsInput {
            filter: Some("attention".into()),
            known_recording_ids: known,
            ..Default::default()
        },
        &storage,
    )
    .unwrap();
    assert!(empty.groups.is_empty());
    assert_eq!(empty.reconciled_groups.len(), 3);
    assert_eq!(empty.counts.all, 200);
    storage
        .with_connection(|connection| {
            assert_eq!(
                connection
                    .query_row("SELECT COUNT(*) FROM recall_source_files", [], |row| row
                        .get::<_, i64>(0))?,
                5
            );
            assert!(connection.query_row(
                "SELECT source_dirty FROM recall_meta WHERE id=1",
                [],
                |row| row.get::<_, bool>(0)
            )?);
            // A later global owner must still process all outstanding source changes.
            storage::refresh_evidence(connection)?;
            assert_eq!(
                connection
                    .query_row("SELECT COUNT(*) FROM recall_source_files", [], |row| row
                        .get::<_, i64>(0))?,
                200
            );
            assert_eq!(
                connection.query_row(
                    "SELECT COUNT(*) FROM recall_dirty_recordings",
                    [],
                    |row| row.get::<_, i64>(0)
                )?,
                0
            );
            Ok(())
        })
        .unwrap();
}
