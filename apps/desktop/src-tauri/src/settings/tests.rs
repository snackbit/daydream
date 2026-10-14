use std::path::PathBuf;

use crate::storage::{self, SaveAppSettingInput};

use super::validation::validate_settings;
use super::*;

#[test]
fn fast_local_preset_preserves_unrelated_preferences_and_sensitive_settings() {
    let storage = crate::storage::test_storage(std::env::temp_dir());
    storage
        .with_connection(|connection| {
            crate::storage::save_app_setting(
                connection,
                &crate::storage::SaveAppSettingInput {
                    key: "ai.ocr.engine".into(),
                    value_json: "\"vision\"".into(),
                    is_sensitive: false,
                    timestamp_ms: 1,
                },
            )?;
            crate::storage::save_app_setting(
                connection,
                &crate::storage::SaveAppSettingInput {
                    key: "synthetic-sensitive-setting".into(),
                    value_json: "\"synthetic-fixture-only\"".into(),
                    is_sensitive: true,
                    timestamp_ms: 1,
                },
            )?;
            Ok(())
        })
        .unwrap();
    let original = load_app_settings(&storage).unwrap();
    use_fast_local_processing(&storage).unwrap();
    let updated = load_app_settings(&storage).unwrap();
    assert_eq!(updated.ai.ocr.engine, "tesseract");
    assert_eq!(updated.ai.processing.visual_sampling_mode, "adaptive-v1");
    assert_eq!(updated.ai.processing.resource_mode, "fast");
    assert_eq!(updated.ai.processing.max_parallel_ocr_jobs, 4);
    assert_eq!(updated.ai.vision, original.ai.vision);
    assert_eq!(updated.ai.transcription, original.ai.transcription);
    assert_eq!(updated.ai.recall, original.ai.recall);
    assert_eq!(updated.record, original.record);
    assert_eq!(updated.storage, original.storage);
    assert!(
        storage
            .with_connection(|connection| crate::storage::get_app_setting(
                connection,
                "synthetic-sensitive-setting"
            ))
            .unwrap()
            .unwrap()
            .is_sensitive
    );
}

#[test]
fn incomplete_vision_profile_can_be_saved_before_credentials_are_entered() {
    let mut settings = default_settings();
    settings.ai.ocr.engine = "vision".into();
    settings.ai.vision.provider = "openai-compatible".into();
    settings.ai.vision.base_url = Some("http://127.0.0.1:8080/v1".into());
    settings.ai.vision.model = Some("synthetic-vision".into());
    assert!(
        validation::validate_settings(settings).is_ok(),
        "An incomplete profile must not block unrelated settings or the first key save"
    );
}

#[test]
fn vision_ocr_is_selectable_without_enabling_screen_observations() {
    let mut settings = default_settings();
    settings.ai.ocr.engine = "vision".into();
    settings.ai.vision.provider = "openai-compatible".into();
    settings.ai.vision.base_url = Some("http://127.0.0.1:8080".into());
    settings.ai.vision.api_key_ref = Some("ai.secret.vision.test".into());
    settings.ai.vision.model = Some("vision-test".into());
    assert!(validate_settings(settings).is_ok());
}

#[test]
fn legacy_provider_configuration_migrates_independent_observation_flag() {
    for provider in ["disabled", "openai-compatible"] {
        let rows = vec![storage::AppSettingRow {
            key: "ai.vision.provider".into(),
            value_json: serde_json::to_string(provider).unwrap(),
            is_sensitive: false,
            created_at_ms: 1,
            updated_at_ms: 1,
        }];
        let migrated = super::persistence::settings_from_storage_rows(rows).unwrap();
        assert_eq!(
            migrated.ai.vision.observations_enabled,
            provider == "openai-compatible"
        );
        assert_eq!(migrated.ai.ocr.engine, "tesseract");
    }
}

#[test]
fn load_app_settings_returns_defaults_without_saved_row() {
    let storage = storage::test_storage(PathBuf::from("/tmp/daydream.sqlite3"));
    let settings = load_app_settings(&storage).expect("load settings");

    assert_eq!(settings, default_settings());
}

#[test]
fn visual_frame_parallel_defaults_track_cpu_limits() {
    let defaults = default_settings();
    let limits = settings_limits();
    let cpu_count = available_cpu_core_count();

    assert_eq!(
        defaults.ai.processing.max_parallel_ocr_jobs,
        default_visual_frame_parallel_jobs()
    );
    assert_eq!(
        defaults.ai.processing.max_parallel_vlm_jobs,
        default_visual_frame_parallel_jobs()
    );
    assert!(defaults.ai.processing.max_parallel_ocr_jobs >= 1);
    assert!(defaults.ai.processing.max_parallel_ocr_jobs <= cpu_count);
    assert_eq!(limits.ai.processing.max_parallel_ocr_jobs_min, 1);
    assert_eq!(limits.ai.processing.max_parallel_vlm_jobs_min, 1);
    assert_eq!(limits.ai.processing.max_parallel_ocr_jobs_max, cpu_count);
    assert_eq!(limits.ai.processing.max_parallel_vlm_jobs_max, cpu_count);
}

#[test]
fn load_app_settings_returns_saved_values() {
    let storage = storage::test_storage(PathBuf::from("/tmp/daydream.sqlite3"));
    storage
        .with_connection_mut(|connection| {
            storage::save_app_settings(
                connection,
                &[
                    SaveAppSettingInput {
                        key: "record.fps".to_string(),
                        value_json: "5.0".to_string(),
                        is_sensitive: false,
                        timestamp_ms: 1,
                    },
                    SaveAppSettingInput {
                        key: "record.videoBitrateKbps".to_string(),
                        value_json: "2500".to_string(),
                        is_sensitive: false,
                        timestamp_ms: 1,
                    },
                    SaveAppSettingInput {
                        key: "storage.retentionDays".to_string(),
                        value_json: "30".to_string(),
                        is_sensitive: false,
                        timestamp_ms: 1,
                    },
                    SaveAppSettingInput {
                        key: "storage.sizeLimitBytes".to_string(),
                        value_json: (75_u64 * 1024 * 1024 * 1024).to_string(),
                        is_sensitive: false,
                        timestamp_ms: 1,
                    },
                    SaveAppSettingInput {
                        key: "ai.transcription.language".to_string(),
                        value_json: "\"de\"".to_string(),
                        is_sensitive: false,
                        timestamp_ms: 1,
                    },
                ],
            )
        })
        .expect("save settings");

    let settings = load_app_settings(&storage).expect("load settings");

    assert_eq!(settings.record.fps, 5.0);
    assert_eq!(settings.record.video_bitrate_kbps, 2_500);
    assert_eq!(settings.storage.retention_days, 30);
    assert_eq!(settings.storage.size_limit_bytes, 75 * 1024 * 1024 * 1024);
    assert_eq!(settings.ai.transcription.language, "de");
}

#[test]
fn load_app_settings_handles_saved_vulkan_by_build_feature() {
    let storage = storage::test_storage(PathBuf::from("/tmp/daydream.sqlite3"));
    storage
        .with_connection_mut(|connection| {
            storage::save_app_setting(
                connection,
                &SaveAppSettingInput {
                    key: "ai.transcription.localBackend".to_string(),
                    value_json: "\"vulkan\"".to_string(),
                    is_sensitive: false,
                    timestamp_ms: 1,
                },
            )
        })
        .expect("save settings");

    let settings = load_app_settings(&storage).expect("load settings");

    if cfg!(feature = "whisper-vulkan") {
        assert_eq!(settings.ai.transcription.local_backend, "vulkan");
    } else {
        assert_eq!(
            settings.ai.transcription.local_backend,
            DEFAULT_AI_TRANSCRIPTION_BACKEND
        );
    }
}

#[test]
fn validate_settings_rejects_out_of_range_values() {
    for (settings, field) in [
        (
            AppSettings {
                record: RecordSettings {
                    fps: 0.0,
                    ..default_settings().record
                },
                ..default_settings()
            },
            "record.fps",
        ),
        (
            AppSettings {
                record: RecordSettings {
                    video_bitrate_kbps: 10_001,
                    ..default_settings().record
                },
                ..default_settings()
            },
            "record.videoBitrateKbps",
        ),
        (
            AppSettings {
                storage: StorageSettings {
                    retention_days: 366,
                    ..default_settings().storage
                },
                ..default_settings()
            },
            "storage.retentionDays",
        ),
        (
            AppSettings {
                storage: StorageSettings {
                    size_limit_bytes: MIN_STORAGE_SIZE_LIMIT_BYTES - 1,
                    ..default_settings().storage
                },
                ..default_settings()
            },
            "storage.sizeLimitBytes",
        ),
    ] {
        let error =
            validate_settings(settings).expect_err("invalid settings should fail validation");

        assert_eq!(error.field, Some(field.to_string()));
    }
}

#[test]
fn validate_settings_rejects_invalid_ai_values() {
    for (settings, field) in [
        (
            AppSettings {
                ai: AiSettings {
                    transcription: AiTranscriptionProviderSettings {
                        provider: "remote".to_string(),
                        ..default_settings().ai.transcription
                    },
                    ..default_settings().ai
                },
                ..default_settings()
            },
            "ai.transcription.provider",
        ),
        (
            AppSettings {
                ai: AiSettings {
                    transcription: AiTranscriptionProviderSettings {
                        local_model_id: "small.en".to_string(),
                        ..default_settings().ai.transcription
                    },
                    ..default_settings().ai
                },
                ..default_settings()
            },
            "ai.transcription.localModelId",
        ),
        (
            AppSettings {
                ai: AiSettings {
                    transcription: AiTranscriptionProviderSettings {
                        local_backend: "metal".to_string(),
                        ..default_settings().ai.transcription
                    },
                    ..default_settings().ai
                },
                ..default_settings()
            },
            "ai.transcription.localBackend",
        ),
        (
            AppSettings {
                ai: AiSettings {
                    transcription: AiTranscriptionProviderSettings {
                        language: "klingon".to_string(),
                        ..default_settings().ai.transcription
                    },
                    ..default_settings().ai
                },
                ..default_settings()
            },
            "ai.transcription.language",
        ),
        (
            AppSettings {
                ai: AiSettings {
                    transcription: AiTranscriptionProviderSettings {
                        remote_base_url: Some("file:///tmp/model".to_string()),
                        ..default_settings().ai.transcription
                    },
                    ..default_settings().ai
                },
                ..default_settings()
            },
            "ai.transcription.remoteBaseUrl",
        ),
        (
            AppSettings {
                ai: AiSettings {
                    transcription: AiTranscriptionProviderSettings {
                        max_initial_timestamp_seconds:
                            MAX_AI_TRANSCRIPTION_MAX_INITIAL_TIMESTAMP_SECONDS + 1.0,
                        ..default_settings().ai.transcription
                    },
                    ..default_settings().ai
                },
                ..default_settings()
            },
            "ai.transcription.maxInitialTimestampSeconds",
        ),
        (
            AppSettings {
                ai: AiSettings {
                    transcription: AiTranscriptionProviderSettings {
                        silence_threshold_db: MIN_AI_TRANSCRIPTION_SILENCE_THRESHOLD_DB - 1.0,
                        ..default_settings().ai.transcription
                    },
                    ..default_settings().ai
                },
                ..default_settings()
            },
            "ai.transcription.silenceThresholdDb",
        ),
        (
            AppSettings {
                ai: AiSettings {
                    transcription: AiTranscriptionProviderSettings {
                        min_speech_ms: MIN_AI_TRANSCRIPTION_MIN_SPEECH_MS - 1,
                        ..default_settings().ai.transcription
                    },
                    ..default_settings().ai
                },
                ..default_settings()
            },
            "ai.transcription.minSpeechMs",
        ),
        (
            AppSettings {
                ai: AiSettings {
                    transcription: AiTranscriptionProviderSettings {
                        merge_gap_ms: MAX_AI_TRANSCRIPTION_MERGE_GAP_MS + 1,
                        ..default_settings().ai.transcription
                    },
                    ..default_settings().ai
                },
                ..default_settings()
            },
            "ai.transcription.mergeGapMs",
        ),
        (
            AppSettings {
                ai: AiSettings {
                    processing: AiProcessingSettings {
                        max_parallel_jobs: MAX_AI_MAX_PARALLEL_JOBS + 1,
                        ..default_settings().ai.processing
                    },
                    ..default_settings().ai
                },
                ..default_settings()
            },
            "ai.processing.maxParallelJobs",
        ),
        (
            AppSettings {
                ai: AiSettings {
                    processing: AiProcessingSettings {
                        frame_recognition_fps: MIN_AI_FRAME_RECOGNITION_FPS / 2.0,
                        ..default_settings().ai.processing
                    },
                    ..default_settings().ai
                },
                ..default_settings()
            },
            "ai.processing.frameRecognitionFps",
        ),
        (
            AppSettings {
                ai: AiSettings {
                    processing: AiProcessingSettings {
                        periodic_full_frame_interval_ms: MIN_AI_PERIODIC_FULL_FRAME_INTERVAL_MS - 1,
                        ..default_settings().ai.processing
                    },
                    ..default_settings().ai
                },
                ..default_settings()
            },
            "ai.processing.periodicFullFrameIntervalMs",
        ),
        (
            AppSettings {
                ai: AiSettings {
                    processing: AiProcessingSettings {
                        max_full_frame_checkpoints_per_minute:
                            MAX_AI_MAX_FULL_FRAME_CHECKPOINTS_PER_MINUTE + 1,
                        ..default_settings().ai.processing
                    },
                    ..default_settings().ai
                },
                ..default_settings()
            },
            "ai.processing.maxFullFrameCheckpointsPerMinute",
        ),
        (
            AppSettings {
                ai: AiSettings {
                    processing: AiProcessingSettings {
                        visual_delta_tile_size_px: MIN_AI_VISUAL_DELTA_TILE_SIZE_PX - 1,
                        ..default_settings().ai.processing
                    },
                    ..default_settings().ai
                },
                ..default_settings()
            },
            "ai.processing.visualDeltaTileSizePx",
        ),
    ] {
        let error =
            validate_settings(settings).expect_err("invalid AI settings should fail validation");

        assert_eq!(error.field, Some(field.to_string()));
    }
}
