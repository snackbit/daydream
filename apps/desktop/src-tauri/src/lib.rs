mod ai;
mod diagnostics;
mod ffmpeg;
mod library;
mod notifications;
mod playback_server;
pub mod recorder;
pub mod secret_crypto;
mod settings;
pub mod storage;
mod storage_limits;
mod telemetry;

use tauri::Manager;
use tracing::warn;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            telemetry::initialize(app.handle())?;
            let recorder_manager = recorder::RecorderManager::new(app.path().app_data_dir()?);
            let storage = storage::initialize(app.handle())?;
            match settings::load_app_settings(&storage) {
                Ok(app_settings) => {
                    let cleanup = storage_limits::enforce_storage_cleanup(
                        &storage,
                        &app.path().app_data_dir()?.join("recordings"),
                        &app_settings.storage,
                    );
                    if cleanup.failed_recording_count > 0 {
                        warn!(
                            failed_recording_count = cleanup.failed_recording_count,
                            "Startup storage cleanup had failures."
                        );
                    }
                }
                Err(_error) => {
                    warn!("Startup storage cleanup skipped because settings failed to load.")
                }
            }
            let playback_server = playback_server::PlaybackServer::start()?;
            app.manage(storage);
            app.manage(recorder_manager);
            app.manage(playback_server);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            library::close_library_playback_session,
            library::create_library_playback_session,
            diagnostics::get_diagnostics,
            diagnostics::get_debug_log_bundle,
            library::delete_library_recording,
            library::delete_library_recordings,
            library::get_library_recording,
            library::get_recording_waveform_tiles,
            library::get_recording_waveforms,
            library::list_library_recordings,
            library::prepare_library_recording_thumbnail,
            settings::get_app_settings,
            settings::preview_app_settings_storage_cleanup,
            settings::save_app_settings,
            settings::reset_app_settings,
            notifications::get_notifications,
            notifications::acknowledge_notifications,
            notifications::dismiss_notification,
            notifications::delete_notifications,
            notifications::delete_all_notifications,
            notifications::execute_notification_action,
            ai::list_ai_models,
            ai::download_ai_model,
            ai::delete_ai_model,
            ai::self_test_ai_model,
            ai::get_ai_provider_status,
            ai::save_ai_provider_secret,
            ai::delete_ai_provider_secret,
            ai::list_processing_jobs,
            ai::pause_processing_job,
            ai::resume_processing_job,
            ai::cancel_processing_job,
            ai::retry_processing_job,
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
