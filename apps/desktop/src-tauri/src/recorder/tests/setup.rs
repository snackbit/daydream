use super::*;
use crate::recorder::source_recording::{persist_capture_setup, SaveCaptureSetupInput};

#[test]
fn reviewed_setup_is_saved_atomically_and_failed_persistence_keeps_previous_setup() {
    let storage = test_storage();
    let id = Uuid::now_v7().to_string();
    let mut scene = test_scene(&id);
    scene.independent_video = true;
    let input = SaveCaptureSetupInput {
        scene: scene.clone(),
        fps: 0.2,
        video_bitrate_kbps: 2000,
    };
    persist_capture_setup(input.clone(), &storage).unwrap();
    storage.with_connection(|connection| {
        connection.execute_batch("CREATE TRIGGER reject_capture_settings BEFORE UPDATE ON app_settings WHEN NEW.key='record.fps' BEGIN SELECT RAISE(ABORT,'synthetic persistence failure'); END;")
    }).unwrap();
    let mut edited = input;
    edited.scene.name = "Edited synthetic source".into();
    edited.fps = 2.0;
    edited.video_bitrate_kbps = 4000;
    assert!(persist_capture_setup(edited, &storage).is_err());
    let saved = storage
        .with_connection(|connection| storage::get_recorder_scene(connection, &id))
        .unwrap();
    assert_eq!(saved.name, scene.name);
    let settings = crate::settings::load_app_settings(&storage).unwrap();
    assert_eq!(settings.record.fps, 0.2);
    assert_eq!(settings.record.video_bitrate_kbps, 2000);
}
