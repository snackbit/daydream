mod ai;
mod background_tasks;
mod diagnostics;
mod ffmpeg;
mod library;
mod maintenance;
mod model_registry;
mod notifications;
mod playback_server;
mod provider_http;
mod recall;
pub mod recorder;
mod recording_deletion;
mod recording_privacy;
mod redaction_targets;
pub mod secret_crypto;
mod settings;
mod startup;
pub mod storage;
mod storage_limits;
mod subprocess;
mod telemetry;
mod timeline_lanes;
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
            let recording_data_lease =
                recording_deletion::acquire_recording_data_lease(&app_data_dir)
                    .map_err(std::io::Error::other)?;
            app.manage(recording_data_lease);
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
            let runtime_registry = maintenance::RuntimeActivityRegistry::default();
            let maintenance_owner = maintenance::commands::initialize(&storage, &runtime_registry)
                .map_err(std::io::Error::other)?;
            let maintenance_pending = runtime_registry.is_maintenance_active();
            app.manage(runtime_registry);
            app.manage(maintenance_owner);
            if !maintenance_pending {
                recording_deletion::remove_recovery_outputs(
                    &storage,
                    &app_data_dir.join("recordings"),
                    None,
                )
                .map_err(std::io::Error::other)?;
                recording_deletion::remove_stale_processing_artifacts(&app_data_dir)
                    .map_err(std::io::Error::other)?;
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
            }
            let playback_server = playback_server::PlaybackServer::start()?;
            playback_server.attach_runtime_registry(
                app.state::<maintenance::RuntimeActivityRegistry>()
                    .inner()
                    .clone(),
            );
            app.manage(storage);
            app.manage(ai_model_manager);
            app.manage(ai::ProcessingWorkerOwners::default());
            app.manage(transcription_provider_manager);
            app.manage(recorder_manager);
            app.manage(playback_server);
            app.manage(startup_manager);
            app.manage(recall::search_service::SearchControl::default());
            if !maintenance_pending {
                ai::recover_model_downloads(app.handle().clone());
            }
            ai::commands::worker_startup::start_processing_worker(app.handle().clone());
            recall::worker::start_worker(app.handle().clone());
            recall::mutations::start_revision_observer(app.handle().clone());
            startup::start_app_startup(app.handle().clone());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            telemetry::failures::log_frontend_failure,
            model_registry::commands::test_configured_model,
            model_registry::commands::cancel_model_request,
            model_registry::commands::sign_in_chatgpt,
            model_registry::commands::prepare_chatgpt_connection,
            model_registry::commands::get_chatgpt_connection,
            model_registry::commands::sign_out_chatgpt,
            maintenance::commands::preview_maintenance,
            maintenance::commands::execute_maintenance,
            maintenance::commands::get_maintenance_status,
            maintenance::commands::acknowledge_maintenance_theme_reset,
            recall::commands::list_recall_days,
            recall::commands::get_recall_day,
            recall::commands::search_recall_evidence,
            recall::commands::cancel_recall_search,
            recall::commands::resolve_recall_evidence,
            recall::commands::regenerate_recall_day,
            recall::commands::set_recall_feedback,
            recall::commands::set_recall_session_correction,
            recall::commands::accept_recall_suggestion,
            recall::commands::dismiss_recall_suggestion,
            recall::commands::update_recall_task,
            recall::commands::set_recall_day_reviewed,
            recall::commands::get_recall_status,
            recall::commands::test_recall_provider,
            recall::commands::control_recall_job,
            background_tasks::list_background_tasks,
            background_tasks::get_background_task,
            background_tasks::group_commands::list_recording_processing_groups,
            background_tasks::group_commands::get_recording_processing_lane,
            background_tasks::group_commands::control_recording_processing_groups,
            ai::commands::processing_commands::retry_processing_units,
            ai::commands::processing_commands::reprocess_recording_transcription,
            ai::commands::reprocessing_commands::preview_recording_reprocessing,
            ai::commands::reprocessing_commands::start_recording_reprocessing,
            startup::get_app_startup_status,
            startup::retry_app_startup,
            library::commands::close_library_playback_session,
            library::commands::create_library_playback_session,
            library::commands::create_library_redactions,
            library::redaction_resize::resize_library_redaction,
            library::frame_navigation::adjacent_library_frame,
            diagnostics::commands::get_diagnostics,
            diagnostics::commands::get_debug_log_bundle,
            library::commands::delete_library_recording,
            library::commands::delete_library_recordings,
            library::commands::delete_library_redaction,
            library::commands::get_library_recording_debug,
            library::commands::get_library_recording,
            library::audio_cleanup::commands::get_audio_cleanup,
            library::audio_cleanup::commands::apply_audio_cleanup,
            library::audio_cleanup::commands::cancel_audio_cleanup,
            library::audio_cleanup::commands::preview_audio_cleanup,
            library::day::get_library_day_timeline,
            ai::commands::result_commands::get_recording_activity,
            ai::commands::result_commands::get_recording_ai_processing_state,
            ai::commands::result_commands::get_recording_transcript,
            ai::commands::result_commands::get_recording_transcript_preview,
            ai::commands::result_commands::get_recording_visual_observations,
            ai::commands::ocr_commands::get_ocr_document,
            ai::commands::ocr_commands::get_ocr_history,
            ai::commands::ocr_commands::get_ocr_timeline_frames,
            ai::commands::ocr_commands::reprocess_recording_ocr,
            library::commands::get_recording_waveform_tiles,
            library::commands::get_recording_waveforms,
            library::commands::list_library_recordings,
            library::day::listing::list_library_days,
            library::commands::prepare_library_recording_thumbnail,
            library::commands::rename_library_recording,
            recorder::recovery::recover_library_recording,
            library::commands::unredact_library_redactions,
            settings::commands::get_app_settings,
            settings::commands::get_input_activity_status,
            settings::commands::preview_app_settings_storage_cleanup,
            settings::commands::repair_input_activity_permissions,
            settings::commands::save_app_settings,
            settings::commands::reset_app_settings,
            notifications::get_notifications,
            notifications::acknowledge_notifications,
            notifications::dismiss_notification,
            notifications::delete_notifications,
            notifications::delete_all_notifications,
            notifications::execute_notification_action,
            ai::commands::model_commands::list_ai_models,
            ai::commands::model_commands::download_ai_model,
            ai::commands::model_commands::cancel_ai_model_download,
            ai::commands::model_commands::pause_ai_model_download,
            ai::commands::model_commands::resume_ai_model_download,
            ai::commands::model_commands::remove_ai_model_partial,
            ai::commands::model_commands::delete_ai_model,
            ai::commands::model_commands::self_test_ai_model,
            ai::commands::provider_commands::get_ai_provider_status,
            ai::processing::provider_test::test_vision_provider,
            ai::commands::provider_commands::save_ai_provider_secret,
            ai::commands::provider_commands::delete_ai_provider_secret,
            ai::commands::processing_commands::list_processing_jobs,
            ai::commands::processing_commands::start_recording_ai_processing,
            ai::commands::processing_commands::pause_processing_job,
            ai::commands::processing_commands::resume_processing_job,
            ai::commands::processing_commands::cancel_processing_job,
            ai::commands::processing_commands::retry_processing_job,
            recorder::get_recorder_status,
            recorder::get_recorder_performance_stats,
            recorder::get_recorder_recording_stats,
            recorder::list_protectable_windows,
            recorder::get_window_application_icon,
            recorder::get_window_protection_status,
            recorder::get_capture_sources,
            recorder::get_recorder_scene,
            recorder::select_capture_sources,
            recorder::get_audio_sources,
            recorder::save_recorder_scene,
            recorder::source_recording::get_capture_setup,
            recorder::source_recording::save_capture_setup,
            recorder::get_recorder_preview_status,
            recorder::start_recorder_preview,
            recorder::stop_recorder_preview,
            recorder::update_recorder_preview_demand,
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

#[cfg(target_os = "linux")]
pub fn run_media_worker_process(control_fd: i32, frames_fd: i32) -> i32 {
    recorder::media_worker::run_worker_process(control_fd, frames_fd)
}
