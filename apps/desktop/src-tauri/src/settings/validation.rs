use crate::whisper_backend;

use super::{defaults::*, types::*};

pub(super) fn validate_settings(settings: AppSettings) -> Result<AppSettings, AppSettingsError> {
    validate_f64_range(
        "record.fps",
        settings.record.fps,
        MIN_RECORD_FPS,
        MAX_RECORD_FPS,
        "Frame rate must be between one frame every 60 seconds and 5 frames per second.",
    )?;
    validate_u32_range(
        "record.videoBitrateKbps",
        settings.record.video_bitrate_kbps,
        MIN_VIDEO_BITRATE_KBPS,
        MAX_VIDEO_BITRATE_KBPS,
        "Video bitrate must be between 250 and 10000 kbps.",
    )?;
    validate_u32_range(
        "storage.retentionDays",
        settings.storage.retention_days,
        MIN_STORAGE_RETENTION_DAYS,
        MAX_STORAGE_RETENTION_DAYS,
        "Retention must be between 1 and 365 days.",
    )?;
    if settings.storage.size_limit_bytes < MIN_STORAGE_SIZE_LIMIT_BYTES {
        return Err(validation_error(
            "storage.sizeLimitBytes",
            "Storage size limit must be at least 1 GiB.",
        ));
    }
    validate_ai_settings(&settings.ai)?;

    Ok(settings)
}

fn validate_ai_settings(settings: &AiSettings) -> Result<(), AppSettingsError> {
    validate_one_of(
        "ai.transcription.provider",
        &settings.transcription.provider,
        &["local-whisper-cpp", "openai-compatible"],
        "Transcription provider must be local Whisper or OpenAI-compatible.",
    )?;
    validate_one_of(
        "ai.transcription.localModelId",
        &settings.transcription.local_model_id,
        &[
            "tiny",
            "base",
            "small",
            "medium",
            "large-v3-turbo",
            "large-v3",
        ],
        "Local transcription model must be one of the supported multilingual models.",
    )?;
    validate_transcription_backend(&settings.transcription.local_backend)?;
    validate_transcription_language(&settings.transcription.language)?;
    validate_f64_range(
        "ai.transcription.maxInitialTimestampSeconds",
        settings.transcription.max_initial_timestamp_seconds,
        MIN_AI_TRANSCRIPTION_MAX_INITIAL_TIMESTAMP_SECONDS,
        MAX_AI_TRANSCRIPTION_MAX_INITIAL_TIMESTAMP_SECONDS,
        "Max initial timestamp must be between 1 and 10 seconds.",
    )?;
    validate_u32_range(
        "ai.transcription.mergeGapMs",
        settings.transcription.merge_gap_ms,
        MIN_AI_TRANSCRIPTION_MERGE_GAP_MS,
        MAX_AI_TRANSCRIPTION_MERGE_GAP_MS,
        "Transcription merge gap must be between 0 and 5000 ms.",
    )?;
    validate_u32_range(
        "ai.transcription.minSpeechMs",
        settings.transcription.min_speech_ms,
        MIN_AI_TRANSCRIPTION_MIN_SPEECH_MS,
        MAX_AI_TRANSCRIPTION_MIN_SPEECH_MS,
        "Minimum speech duration must be between 50 and 2000 ms.",
    )?;
    validate_optional_url(
        "ai.transcription.remoteBaseUrl",
        settings.transcription.remote_base_url.as_deref(),
    )?;
    validate_optional_text(
        "ai.transcription.remoteModel",
        settings.transcription.remote_model.as_deref(),
        120,
    )?;
    validate_f64_range(
        "ai.transcription.silenceThresholdDb",
        settings.transcription.silence_threshold_db,
        MIN_AI_TRANSCRIPTION_SILENCE_THRESHOLD_DB,
        MAX_AI_TRANSCRIPTION_SILENCE_THRESHOLD_DB,
        "Silence threshold must be between -80 and -20 dBFS.",
    )?;
    validate_one_of(
        "ai.vision.provider",
        &settings.vision.provider,
        &["disabled", "openai-compatible"],
        "Vision provider must be disabled or OpenAI-compatible.",
    )?;
    validate_optional_url("ai.vision.baseUrl", settings.vision.base_url.as_deref())?;
    validate_one_of(
        "ai.ocr.engine",
        &settings.ocr.engine,
        &["tesseract"],
        "OCR engine must be Tesseract.",
    )?;
    validate_u32_range(
        "ai.processing.maxParallelJobs",
        settings.processing.max_parallel_jobs,
        MIN_AI_MAX_PARALLEL_JOBS,
        MAX_AI_MAX_PARALLEL_JOBS,
        "Max parallel jobs must be between 1 and 4.",
    )?;
    validate_u32_range(
        "ai.processing.maxParallelChunksPerJob",
        settings.processing.max_parallel_chunks_per_job,
        MIN_AI_MAX_PARALLEL_CHUNKS_PER_JOB,
        MAX_AI_MAX_PARALLEL_CHUNKS_PER_JOB,
        "Max parallel chunks per job must be between 1 and 8.",
    )?;
    validate_f64_range(
        "ai.processing.frameRecognitionFps",
        settings.processing.frame_recognition_fps,
        MIN_AI_FRAME_RECOGNITION_FPS,
        MAX_AI_FRAME_RECOGNITION_FPS,
        "Frame recognition FPS must be between 1/60 and 5.",
    )?;
    validate_u32_range(
        "ai.processing.periodicFullFrameIntervalMs",
        settings.processing.periodic_full_frame_interval_ms,
        MIN_AI_PERIODIC_FULL_FRAME_INTERVAL_MS,
        MAX_AI_PERIODIC_FULL_FRAME_INTERVAL_MS,
        "Periodic full-frame interval must be between 5 seconds and 10 minutes.",
    )?;
    validate_u32_range(
        "ai.processing.maxFullFrameCheckpointsPerMinute",
        settings.processing.max_full_frame_checkpoints_per_minute,
        MIN_AI_MAX_FULL_FRAME_CHECKPOINTS_PER_MINUTE,
        MAX_AI_MAX_FULL_FRAME_CHECKPOINTS_PER_MINUTE,
        "Max full-frame checkpoints must be between 1 and 60 per minute.",
    )?;
    validate_u32_range(
        "ai.processing.visualDeltaTileSizePx",
        settings.processing.visual_delta_tile_size_px,
        MIN_AI_VISUAL_DELTA_TILE_SIZE_PX,
        MAX_AI_VISUAL_DELTA_TILE_SIZE_PX,
        "Visual delta tile size must be between 32 and 256 px.",
    )?;
    validate_u32_range(
        "ai.activity.keyboard.windowMs",
        settings.activity.keyboard.window_ms,
        MIN_AI_ACTIVITY_WINDOW_MS,
        MAX_AI_ACTIVITY_WINDOW_MS,
        "Keyboard activity window must be between 1 second and 5 minutes.",
    )?;
    validate_u32_range(
        "ai.activity.mouse.windowMs",
        settings.activity.mouse.window_ms,
        MIN_AI_ACTIVITY_WINDOW_MS,
        MAX_AI_ACTIVITY_WINDOW_MS,
        "Mouse activity window must be between 1 second and 5 minutes.",
    )?;

    Ok(())
}

pub(super) fn coerce_stored_transcription_backend(value: String) -> String {
    if transcription_backend_available(&value) {
        value
    } else {
        DEFAULT_AI_TRANSCRIPTION_BACKEND.to_string()
    }
}

fn validate_transcription_backend(value: &str) -> Result<(), AppSettingsError> {
    validate_one_of(
        "ai.transcription.localBackend",
        value,
        &["cpu", "vulkan", "cuda"],
        "Local transcription backend must be CPU, Vulkan, or CUDA.",
    )?;
    if transcription_backend_available(value) {
        return Ok(());
    }

    Err(validation_error(
        "ai.transcription.localBackend",
        "Selected local transcription backend is not available in this build.",
    ))
}

fn validate_transcription_language(value: &str) -> Result<(), AppSettingsError> {
    if value == DEFAULT_AI_TRANSCRIPTION_LANGUAGE || whisper_backend::language_supported(value) {
        return Ok(());
    }

    Err(validation_error(
        "ai.transcription.language",
        "Transcription language must be auto or one of the supported Whisper languages.",
    ))
}

fn transcription_backend_available(value: &str) -> bool {
    match value {
        "cpu" => true,
        "vulkan" => cfg!(feature = "whisper-vulkan"),
        "cuda" => cfg!(feature = "whisper-cuda"),
        _ => false,
    }
}

fn validate_one_of(
    field: &str,
    value: &str,
    allowed: &[&str],
    message: &str,
) -> Result<(), AppSettingsError> {
    if !allowed.contains(&value) {
        return Err(validation_error(field, message));
    }

    Ok(())
}

fn validate_optional_url(field: &str, value: Option<&str>) -> Result<(), AppSettingsError> {
    let Some(value) = value else {
        return Ok(());
    };

    if value.starts_with("http://") || value.starts_with("https://") {
        return Ok(());
    }

    Err(validation_error(
        field,
        "Remote endpoint URL must start with http:// or https://.",
    ))
}

fn validate_optional_text(
    field: &str,
    value: Option<&str>,
    max_chars: usize,
) -> Result<(), AppSettingsError> {
    let Some(value) = value else {
        return Ok(());
    };
    if value.chars().count() <= max_chars {
        return Ok(());
    }

    Err(validation_error(field, "Setting value is too long."))
}

fn validate_f64_range(
    field: &str,
    value: f64,
    min: f64,
    max: f64,
    message: &str,
) -> Result<(), AppSettingsError> {
    if !value.is_finite() || value < min || value > max {
        return Err(validation_error(field, message));
    }

    Ok(())
}

fn validate_u32_range(
    field: &str,
    value: u32,
    min: u32,
    max: u32,
    message: &str,
) -> Result<(), AppSettingsError> {
    if value < min || value > max {
        return Err(validation_error(field, message));
    }

    Ok(())
}

fn validation_error(field: &str, message: &str) -> AppSettingsError {
    AppSettingsError {
        code: "settings_invalid_value".to_string(),
        field: Some(field.to_string()),
        message: message.to_string(),
    }
}
