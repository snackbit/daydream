use serde::Serialize;

use crate::storage::{AppSettingRow, SaveAppSettingInput};

use super::{
    defaults::default_settings,
    types::*,
    validation::{coerce_stored_transcription_backend, validate_settings},
};

pub(super) fn settings_from_storage_rows(
    rows: Vec<AppSettingRow>,
) -> Result<AppSettings, AppSettingsError> {
    let mut settings = default_settings();
    let observations_flag_present = rows
        .iter()
        .any(|row| row.key == "ai.vision.observationsEnabled");

    for row in rows {
        apply_setting_row(&mut settings, &row)?;
    }
    if !observations_flag_present {
        settings.ai.vision.observations_enabled =
            settings.ai.vision.provider == "openai-compatible";
    }

    validate_settings(settings)
}

fn apply_setting_row(
    settings: &mut AppSettings,
    row: &AppSettingRow,
) -> Result<(), AppSettingsError> {
    match row.key.as_str() {
        "ai.recall" => settings.ai.recall = setting_value(row)?,
        "ai.vision.observationsEnabled" => {
            settings.ai.vision.observations_enabled = setting_value(row)?
        }
        "debug.enabled" => settings.debug.enabled = setting_value(row)?,
        "expert.enabled" => settings.expert.enabled = setting_value(row)?,
        "record.fps" => settings.record.fps = setting_value(row)?,
        "record.videoBitrateKbps" => settings.record.video_bitrate_kbps = setting_value(row)?,
        "record.windowProtection" => settings.record.window_protection = setting_value(row)?,
        "storage.retentionDays" => settings.storage.retention_days = setting_value(row)?,
        "storage.sizeLimitBytes" => settings.storage.size_limit_bytes = setting_value(row)?,
        "ai.transcription.provider" => settings.ai.transcription.provider = setting_value(row)?,
        "ai.transcription.localModelId" => {
            settings.ai.transcription.local_model_id = setting_value(row)?;
        }
        "ai.transcription.localBackend" => {
            settings.ai.transcription.local_backend =
                coerce_stored_transcription_backend(setting_value(row)?);
        }
        "ai.transcription.language" => {
            settings.ai.transcription.language = setting_value(row)?;
        }
        "ai.transcription.maxInitialTimestampSeconds" => {
            settings.ai.transcription.max_initial_timestamp_seconds = setting_value(row)?;
        }
        "ai.transcription.mergeGapMs" => {
            settings.ai.transcription.merge_gap_ms = setting_value(row)?;
        }
        "ai.transcription.minSpeechMs" => {
            settings.ai.transcription.min_speech_ms = setting_value(row)?;
        }
        "ai.transcription.remoteBaseUrl" => {
            settings.ai.transcription.remote_base_url = setting_value(row)?;
        }
        "ai.transcription.remoteModel" => {
            settings.ai.transcription.remote_model = setting_value(row)?;
        }
        "ai.transcription.remoteApiKeyRef" => {
            settings.ai.transcription.remote_api_key_ref = setting_value(row)?;
        }
        "ai.transcription.silenceGateEnabled" => {
            settings.ai.transcription.silence_gate_enabled = setting_value(row)?;
        }
        "ai.transcription.silenceThresholdDb" => {
            settings.ai.transcription.silence_threshold_db = setting_value(row)?;
        }
        "ai.transcription.wordTimestampsEnabled" => {
            settings.ai.transcription.word_timestamps_enabled = setting_value(row)?;
        }
        "ai.vision.provider" => settings.ai.vision.provider = setting_value(row)?,
        "ai.vision.authMode" => settings.ai.vision.auth_mode = setting_value(row)?,
        "ai.vision.baseUrl" => settings.ai.vision.base_url = setting_value(row)?,
        "ai.vision.apiKeyRef" => settings.ai.vision.api_key_ref = setting_value(row)?,
        "ai.vision.model" => settings.ai.vision.model = setting_value(row)?,
        "ai.ocr.enabled" => settings.ai.ocr.enabled = setting_value(row)?,
        "ai.ocr.engine" => settings.ai.ocr.engine = setting_value(row)?,
        "ai.processing.autoStartAfterRecording" => {
            settings.ai.processing.auto_start_after_recording = setting_value(row)?;
        }
        "ai.processing.maxParallelJobs" => {
            settings.ai.processing.max_parallel_jobs = setting_value(row)?;
        }
        "ai.processing.maxParallelChunksPerJob" => {
            settings.ai.processing.max_parallel_chunks_per_job = setting_value(row)?;
        }
        "ai.processing.maxParallelOcrJobs" => {
            settings.ai.processing.max_parallel_ocr_jobs = setting_value(row)?;
        }
        "ai.processing.frameRecognitionFps" => {
            settings.ai.processing.frame_recognition_fps = setting_value(row)?;
        }
        "ai.processing.frameRecognitionSyncWithRecording" => {
            settings.ai.processing.frame_recognition_sync_with_recording = setting_value(row)?;
        }
        "ai.processing.periodicFullFrameIntervalMs" => {
            settings.ai.processing.periodic_full_frame_interval_ms = setting_value(row)?;
        }
        "ai.processing.frameSimilarityEnabled" => {
            settings.ai.processing.frame_similarity_enabled = setting_value(row)?;
        }
        "ai.processing.maxFullFrameCheckpointsPerMinute" => {
            settings.ai.processing.max_full_frame_checkpoints_per_minute = setting_value(row)?;
        }
        "ai.processing.maxParallelVlmJobs" => {
            settings.ai.processing.max_parallel_vlm_jobs = setting_value(row)?;
        }
        "ai.processing.visualDeltaTileSizePx" => {
            settings.ai.processing.visual_delta_tile_size_px = setting_value(row)?;
        }
        "ai.activity.keyboard.enabled" => {
            settings.ai.activity.keyboard.enabled = setting_value(row)?
        }
        "ai.activity.keyboard.windowMs" => {
            settings.ai.activity.keyboard.window_ms = setting_value(row)?;
        }
        "ai.activity.mouse.enabled" => settings.ai.activity.mouse.enabled = setting_value(row)?,
        "ai.activity.mouse.windowMs" => settings.ai.activity.mouse.window_ms = setting_value(row)?,
        "ai.activity.frameDeltas.enabled" => {
            settings.ai.activity.frame_deltas.enabled = setting_value(row)?;
        }
        "ai.activity.windows.enabled" => {
            settings.ai.activity.windows.enabled = setting_value(row)?;
        }
        _ => {}
    }

    Ok(())
}

fn setting_value<T: serde::de::DeserializeOwned>(
    row: &AppSettingRow,
) -> Result<T, AppSettingsError> {
    serde_json::from_str(&row.value_json).map_err(|error| AppSettingsError {
        code: "settings_invalid_value".to_string(),
        field: Some(row.key.clone()),
        message: format!("Stored setting value is invalid: {error}"),
    })
}

pub(super) fn settings_to_storage_inputs(
    settings: &AppSettings,
    timestamp_ms: i64,
) -> Result<Vec<SaveAppSettingInput>, AppSettingsError> {
    Ok(vec![
        setting_input("ai.recall", &settings.ai.recall, false, timestamp_ms)?,
        setting_input(
            "debug.enabled",
            &settings.debug.enabled,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "expert.enabled",
            &settings.expert.enabled,
            false,
            timestamp_ms,
        )?,
        setting_input("record.fps", &settings.record.fps, false, timestamp_ms)?,
        setting_input(
            "record.videoBitrateKbps",
            &settings.record.video_bitrate_kbps,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "record.windowProtection",
            &settings.record.window_protection,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "storage.retentionDays",
            &settings.storage.retention_days,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "storage.sizeLimitBytes",
            &settings.storage.size_limit_bytes,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "ai.transcription.provider",
            &settings.ai.transcription.provider,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "ai.transcription.localModelId",
            &settings.ai.transcription.local_model_id,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "ai.transcription.localBackend",
            &settings.ai.transcription.local_backend,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "ai.transcription.language",
            &settings.ai.transcription.language,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "ai.transcription.maxInitialTimestampSeconds",
            &settings.ai.transcription.max_initial_timestamp_seconds,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "ai.transcription.mergeGapMs",
            &settings.ai.transcription.merge_gap_ms,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "ai.transcription.minSpeechMs",
            &settings.ai.transcription.min_speech_ms,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "ai.transcription.remoteBaseUrl",
            &settings.ai.transcription.remote_base_url,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "ai.transcription.remoteModel",
            &settings.ai.transcription.remote_model,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "ai.transcription.remoteApiKeyRef",
            &settings.ai.transcription.remote_api_key_ref,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "ai.transcription.silenceGateEnabled",
            &settings.ai.transcription.silence_gate_enabled,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "ai.transcription.silenceThresholdDb",
            &settings.ai.transcription.silence_threshold_db,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "ai.transcription.wordTimestampsEnabled",
            &settings.ai.transcription.word_timestamps_enabled,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "ai.vision.provider",
            &settings.ai.vision.provider,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "ai.vision.authMode",
            &settings.ai.vision.auth_mode,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "ai.vision.observationsEnabled",
            &settings.ai.vision.observations_enabled,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "ai.vision.baseUrl",
            &settings.ai.vision.base_url,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "ai.vision.apiKeyRef",
            &settings.ai.vision.api_key_ref,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "ai.vision.model",
            &settings.ai.vision.model,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "ai.ocr.enabled",
            &settings.ai.ocr.enabled,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "ai.ocr.engine",
            &settings.ai.ocr.engine,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "ai.processing.autoStartAfterRecording",
            &settings.ai.processing.auto_start_after_recording,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "ai.processing.maxParallelJobs",
            &settings.ai.processing.max_parallel_jobs,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "ai.processing.maxParallelChunksPerJob",
            &settings.ai.processing.max_parallel_chunks_per_job,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "ai.processing.maxParallelOcrJobs",
            &settings.ai.processing.max_parallel_ocr_jobs,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "ai.processing.frameRecognitionFps",
            &settings.ai.processing.frame_recognition_fps,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "ai.processing.frameRecognitionSyncWithRecording",
            &settings.ai.processing.frame_recognition_sync_with_recording,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "ai.processing.periodicFullFrameIntervalMs",
            &settings.ai.processing.periodic_full_frame_interval_ms,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "ai.processing.frameSimilarityEnabled",
            &settings.ai.processing.frame_similarity_enabled,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "ai.processing.maxFullFrameCheckpointsPerMinute",
            &settings.ai.processing.max_full_frame_checkpoints_per_minute,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "ai.processing.maxParallelVlmJobs",
            &settings.ai.processing.max_parallel_vlm_jobs,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "ai.processing.visualDeltaTileSizePx",
            &settings.ai.processing.visual_delta_tile_size_px,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "ai.activity.keyboard.enabled",
            &settings.ai.activity.keyboard.enabled,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "ai.activity.keyboard.windowMs",
            &settings.ai.activity.keyboard.window_ms,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "ai.activity.mouse.enabled",
            &settings.ai.activity.mouse.enabled,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "ai.activity.mouse.windowMs",
            &settings.ai.activity.mouse.window_ms,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "ai.activity.frameDeltas.enabled",
            &settings.ai.activity.frame_deltas.enabled,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "ai.activity.windows.enabled",
            &settings.ai.activity.windows.enabled,
            false,
            timestamp_ms,
        )?,
    ])
}

fn setting_input(
    key: &str,
    value: &impl Serialize,
    is_sensitive: bool,
    timestamp_ms: i64,
) -> Result<SaveAppSettingInput, AppSettingsError> {
    Ok(SaveAppSettingInput {
        key: key.to_string(),
        value_json: serde_json::to_string(value).map_err(|error| AppSettingsError {
            code: "settings_invalid_value".to_string(),
            field: Some(key.to_string()),
            message: format!("Setting value could not be serialized: {error}"),
        })?,
        is_sensitive,
        timestamp_ms,
    })
}
