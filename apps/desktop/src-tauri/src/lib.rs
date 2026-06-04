mod diagnostics;
pub mod recorder;
pub mod storage;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let storage = storage::initialize(app.handle())?;
            app.manage(storage);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            diagnostics::get_diagnostics,
            recorder::get_recorder_status,
            recorder::get_capture_sources,
            recorder::get_audio_sources,
            recorder::save_recorder_scene,
            recorder::start_recording,
            recorder::pause_recording,
            recorder::resume_recording,
            recorder::stop_recording,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
