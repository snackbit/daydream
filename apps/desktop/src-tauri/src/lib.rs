mod ai;
mod diagnostics;
mod ffmpeg;
mod library;
mod notifications;
mod playback_server;
pub mod recorder;
pub mod secret_crypto;
mod settings;
mod startup;
pub mod storage;
mod storage_limits;
mod telemetry;
mod transcription;
mod whisper_backend;

use tauri::Manager;
use tracing::warn;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            telemetry::initialize(app.handle())?;
            let app_data_dir = app.path().app_data_dir()?;
            let recorder_manager = recorder::RecorderManager::new(app_data_dir.clone());
            let ai_model_manager = ai::AiModelManager::new();
            let transcription_provider_manager = transcription::TranscriptionProviderManager::new(
                app_data_dir.clone(),
            )
            .map_err(|error| {
                std::io::Error::other(format!(
                    "transcription provider initialization failed: {}: {}",
                    error.code, error.message
                ))
            })?;
            let startup_manager = startup::AppStartupManager::new();
            let storage = storage::initialize(app.handle())?;
            match settings::load_app_settings(&storage) {
                Ok(app_settings) => {
                    let cleanup = storage_limits::enforce_storage_cleanup(
                        &storage,
                        &app_data_dir.join("recordings"),
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
            app.manage(ai_model_manager);
            app.manage(transcription_provider_manager);
            app.manage(recorder_manager);
            app.manage(playback_server);
            app.manage(startup_manager);
            ai::start_processing_worker(app.handle().clone());
            startup::start_app_startup(app.handle().clone());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            startup::get_app_startup_status,
            startup::retry_app_startup,
            library::close_library_playback_session,
            library::create_library_playback_session,
            diagnostics::get_diagnostics,
            diagnostics::get_debug_log_bundle,
            library::delete_library_recording,
            library::delete_library_recordings,
            library::get_library_recording,
            ai::get_recording_activity,
            ai::get_recording_ai_processing_state,
            ai::get_recording_transcript,
            library::get_recording_waveform_tiles,
            library::get_recording_waveforms,
            library::list_library_recordings,
            library::prepare_library_recording_thumbnail,
            library::rename_library_recording,
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
            ai::cancel_ai_model_download,
            ai::delete_ai_model,
            ai::self_test_ai_model,
            ai::get_ai_provider_status,
            ai::save_ai_provider_secret,
            ai::delete_ai_provider_secret,
            ai::list_processing_jobs,
            ai::start_recording_ai_processing,
            ai::pause_processing_job,
            ai::resume_processing_job,
            ai::cancel_processing_job,
            ai::retry_processing_job,
            recorder::get_recorder_status,
            recorder::get_recorder_performance_stats,
            recorder::get_recorder_recording_stats,
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

pub fn run_whisper_worker_process(app_data_dir: std::path::PathBuf) -> i32 {
    transcription::run_whisper_worker_process(app_data_dir)
}
