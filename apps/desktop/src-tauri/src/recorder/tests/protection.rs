use super::*;

#[test]
fn newer_protected_pause_revokes_deferred_automatic_resume_authority() {
    use super::super::manager::PauseOwner;
    let storage = test_storage();
    let app_data_dir = test_app_data_dir();
    let manager = RecorderManager::new(app_data_dir.clone());
    let scene_id = saved_test_scene(&storage);
    let id = start_test_recording(&manager, &storage, start_input(&scene_id), 100)
        .unwrap()
        .active_recording_id
        .unwrap();
    manager
        .begin_pause_with_owner(&id, 200, PauseOwner::Automatic)
        .unwrap();
    let old = manager.automatic_resume_authority(&id).unwrap().unwrap();
    manager
        .begin_pause_with_owner(&id, 201, PauseOwner::Automatic)
        .unwrap();
    manager
        .complete_pause_recording(&storage, &id, 202)
        .unwrap();
    assert_eq!(
        manager
            .begin_auto_resume_with_authority(&id, 203, old)
            .unwrap_err()
            .code,
        "recorder_stale_automatic_resume"
    );
    assert_eq!(manager.status().unwrap().state, RecorderState::Paused);
    let _ = fs::remove_dir_all(app_data_dir);
}

#[test]
fn manual_pause_can_claim_an_already_paused_recording() {
    let storage = test_storage();
    let app_data_dir = test_app_data_dir();
    let manager = RecorderManager::new(app_data_dir.clone());
    let scene_id = saved_test_scene(&storage);
    let started = start_test_recording(&manager, &storage, start_input(&scene_id), 100)
        .expect("start recording");
    let id = started.active_recording_id.expect("recording id");
    pause_test_recording(&manager, &storage, &id, 200).expect("pause");

    let claimed = manager
        .begin_pause_recording(&id, 250)
        .expect("claim manual pause");
    assert_eq!(claimed.state, RecorderState::Paused);
    assert!(claimed.finalization.is_none());
    let _ = fs::remove_dir_all(app_data_dir);
}

#[test]
fn manual_pause_revokes_automatic_resume_after_user_resume_pause_cycle() {
    let storage = test_storage();
    let app_data_dir = test_app_data_dir();
    let manager = RecorderManager::new(app_data_dir.clone());
    let scene_id = saved_test_scene(&storage);
    let id = start_test_recording(&manager, &storage, start_input(&scene_id), 100)
        .unwrap()
        .active_recording_id
        .unwrap();
    manager
        .begin_pause_with_owner(&id, 200, super::super::manager::PauseOwner::Automatic)
        .unwrap();
    manager
        .complete_pause_recording(&storage, &id, 200)
        .unwrap();
    resume_test_recording(&manager, &storage, &id, 250).unwrap();
    pause_test_recording(&manager, &storage, &id, 300).unwrap();
    assert_eq!(
        manager
            .begin_auto_resume_selecting(&id, 350)
            .unwrap_err()
            .code,
        "recorder_manual_pause"
    );
    assert_eq!(manager.status().unwrap().state, RecorderState::Paused);
    let _ = fs::remove_dir_all(app_data_dir);
}

#[test]
fn manual_pause_during_automatic_finalization_does_not_start_duplicate_completion() {
    let storage = test_storage();
    let app_data_dir = test_app_data_dir();
    let manager = RecorderManager::new(app_data_dir.clone());
    let scene_id = saved_test_scene(&storage);
    let id = start_test_recording(&manager, &storage, start_input(&scene_id), 100)
        .unwrap()
        .active_recording_id
        .unwrap();
    assert!(
        manager
            .begin_pause_with_owner(&id, 200, super::super::manager::PauseOwner::Automatic)
            .unwrap()
            .1
    );
    assert!(
        !manager
            .begin_pause_with_owner(&id, 201, super::super::manager::PauseOwner::Manual)
            .unwrap()
            .1
    );
    manager
        .complete_pause_recording(&storage, &id, 202)
        .unwrap();
    assert_eq!(
        manager
            .begin_auto_resume_selecting(&id, 300)
            .unwrap_err()
            .code,
        "recorder_manual_pause"
    );
    let _ = fs::remove_dir_all(app_data_dir);
}

#[cfg(target_os = "linux")]
#[test]
fn manual_or_protected_pause_cancels_in_flight_automatic_resume_before_activation() {
    use super::super::{
        manager::{PauseOwner, RecordingSegmentRuntime},
        media_writer,
    };
    for owner in [PauseOwner::Manual, PauseOwner::Automatic] {
        let storage = test_storage();
        let app_data_dir = test_app_data_dir();
        let manager = RecorderManager::new(app_data_dir.clone());
        let scene_id = saved_test_scene(&storage);
        let id = start_test_recording(&manager, &storage, start_input(&scene_id), 100)
            .unwrap()
            .active_recording_id
            .unwrap();
        manager
            .begin_pause_with_owner(&id, 200, PauseOwner::Automatic)
            .unwrap();
        manager
            .complete_pause_recording(&storage, &id, 200)
            .unwrap();
        let active = manager.begin_auto_resume_selecting(&id, 300).unwrap();
        let mut pending = manager
            .prepare_resume_segment_start(&storage, active, 300)
            .unwrap();
        pending.segment_input.canvas_width = 2;
        pending.segment_input.canvas_height = 2;
        manager
            .create_pending_segment_rows(&storage, &pending)
            .unwrap();
        assert!(!manager.begin_pause_with_owner(&id, 301, owner).unwrap().1);
        let media = media_writer::start_synthetic_segment(&pending.segment_input).unwrap();
        let status = manager
            .activate_pending_segment(
                &storage,
                pending,
                RecordingSegmentRuntime::new(media, None, None),
                302,
            )
            .unwrap();
        assert_eq!(status.state, RecorderState::Paused);
        assert!(status.active_segment_id.is_none());
        if owner == PauseOwner::Manual {
            assert_eq!(
                manager
                    .begin_auto_resume_selecting(&id, 400)
                    .unwrap_err()
                    .code,
                "recorder_manual_pause"
            );
        } else {
            assert!(manager.automatic_resume_authority(&id).unwrap().is_some());
        }
        resume_test_recording(&manager, &storage, &id, 500).unwrap();
        let segments = storage
            .with_connection(|connection| storage::list_recording_segments(connection, &id))
            .unwrap();
        assert_eq!(
            segments
                .iter()
                .map(|segment| segment.segment_index)
                .collect::<Vec<_>>(),
            [1, 2, 3]
        );
        assert_eq!(segments[1].state, "failed");
        assert_eq!(segments[2].state, "recording");
        let _ = fs::remove_dir_all(app_data_dir);
    }
}
