mod diagnostics;
mod library;
pub mod recorder;
pub mod storage;
mod telemetry;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            telemetry::initialize(app.handle())?;
            let recorder_manager = recorder::RecorderManager::new(app.path().app_data_dir()?);
            let storage = storage::initialize(app.handle())?;
            app.manage(storage);
            app.manage(recorder_manager);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            diagnostics::get_diagnostics,
            diagnostics::get_debug_log_bundle,
            library::delete_library_recording,
            library::delete_library_recordings,
            library::get_library_recording,
            library::get_recording_waveforms,
            library::list_library_recordings,
            library::prepare_library_playback_assets,
            library::prepare_library_playback_thumbnails,
            library::prepare_library_recording_thumbnail,
            recorder::get_recorder_status,
            recorder::get_recorder_performance_stats,
            recorder::get_capture_sources,
            recorder::get_recorder_scene,
            recorder::select_capture_sources,
            recorder::get_audio_sources,
            recorder::save_recorder_scene,
            recorder::get_recorder_preview_status,
            recorder::start_recorder_preview,
            recorder::stop_recorder_preview,
            recorder::start_recording,
            recorder::pause_recording,
            recorder::resume_recording,
            recorder::stop_recording,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
