use super::types::*;

pub const DEFAULT_RECORD_FPS: f64 = 0.2;
pub const MIN_RECORD_FPS: f64 = 1.0 / 60.0;
pub const MAX_RECORD_FPS: f64 = 5.0;
pub const DEFAULT_VIDEO_BITRATE_KBPS: u32 = 1_500;
pub const MIN_VIDEO_BITRATE_KBPS: u32 = 250;
pub const MAX_VIDEO_BITRATE_KBPS: u32 = 10_000;
pub const DEFAULT_WINDOW_PROTECTION_DWELL_MS: u32 = 2_000;
pub const MIN_WINDOW_PROTECTION_DWELL_MS: u32 = 250;
pub const MAX_WINDOW_PROTECTION_DWELL_MS: u32 = 30_000;
pub const DEFAULT_WINDOW_PROTECTION_MERGE_GAP_MS: u32 = 5_000;
pub const MIN_WINDOW_PROTECTION_MERGE_GAP_MS: u32 = 0;
pub const MAX_WINDOW_PROTECTION_MERGE_GAP_MS: u32 = 120_000;
pub const MAX_WINDOW_PROTECTION_RULE_COUNT: usize = 100;
pub const MAX_WINDOW_PROTECTION_TEXT_CHARS: usize = 160;
pub const DEFAULT_STORAGE_RETENTION_DAYS: u32 = 7;
pub const MIN_STORAGE_RETENTION_DAYS: u32 = 1;
pub const MAX_STORAGE_RETENTION_DAYS: u32 = 365;
pub const DEFAULT_STORAGE_SIZE_LIMIT_BYTES: u64 = 50 * 1024 * 1024 * 1024;
pub const MIN_STORAGE_SIZE_LIMIT_BYTES: u64 = 1024 * 1024 * 1024;
pub const DEFAULT_AI_TRANSCRIPTION_MODEL_ID: &str = "small";
pub const DEFAULT_AI_TRANSCRIPTION_BACKEND: &str = "cpu";
pub const DEFAULT_AI_TRANSCRIPTION_LANGUAGE: &str = "auto";
pub const DEFAULT_AI_KEYBOARD_WINDOW_MS: u32 = 30_000;
pub const DEFAULT_AI_MOUSE_WINDOW_MS: u32 = 5_000;
pub const MIN_AI_ACTIVITY_WINDOW_MS: u32 = 1_000;
pub const MAX_AI_ACTIVITY_WINDOW_MS: u32 = 300_000;
pub const DEFAULT_AI_MAX_PARALLEL_JOBS: u32 = 1;
pub const MIN_AI_MAX_PARALLEL_JOBS: u32 = 1;
pub const MAX_AI_MAX_PARALLEL_JOBS: u32 = 4;
pub const DEFAULT_AI_MAX_PARALLEL_CHUNKS_PER_JOB: u32 = 1;
pub const MIN_AI_MAX_PARALLEL_CHUNKS_PER_JOB: u32 = 1;
pub const MAX_AI_MAX_PARALLEL_CHUNKS_PER_JOB: u32 = 8;
pub const MIN_AI_MAX_PARALLEL_OCR_JOBS: u32 = 1;
pub const MIN_AI_MAX_PARALLEL_VLM_JOBS: u32 = 1;
pub const DEFAULT_AI_FRAME_RECOGNITION_FPS: f64 = DEFAULT_RECORD_FPS;
pub const MIN_AI_FRAME_RECOGNITION_FPS: f64 = MIN_RECORD_FPS;
pub const MAX_AI_FRAME_RECOGNITION_FPS: f64 = MAX_RECORD_FPS;
pub const DEFAULT_AI_PERIODIC_FULL_FRAME_INTERVAL_MS: u32 = 60_000;
pub const MIN_AI_PERIODIC_FULL_FRAME_INTERVAL_MS: u32 = 5_000;
pub const MAX_AI_PERIODIC_FULL_FRAME_INTERVAL_MS: u32 = 600_000;
pub const DEFAULT_AI_MAX_FULL_FRAME_CHECKPOINTS_PER_MINUTE: u32 = 2;
pub const MIN_AI_MAX_FULL_FRAME_CHECKPOINTS_PER_MINUTE: u32 = 1;
pub const MAX_AI_MAX_FULL_FRAME_CHECKPOINTS_PER_MINUTE: u32 = 60;
pub const DEFAULT_AI_VISUAL_DELTA_TILE_SIZE_PX: u32 = 64;
pub const MIN_AI_VISUAL_DELTA_TILE_SIZE_PX: u32 = 32;
pub const MAX_AI_VISUAL_DELTA_TILE_SIZE_PX: u32 = 256;
pub const DEFAULT_AI_TRANSCRIPTION_SILENCE_THRESHOLD_DB: f64 = -45.0;
pub const MIN_AI_TRANSCRIPTION_SILENCE_THRESHOLD_DB: f64 = -80.0;
pub const MAX_AI_TRANSCRIPTION_SILENCE_THRESHOLD_DB: f64 = -20.0;
pub const DEFAULT_AI_TRANSCRIPTION_MIN_SPEECH_MS: u32 = 250;
pub const MIN_AI_TRANSCRIPTION_MIN_SPEECH_MS: u32 = 50;
pub const MAX_AI_TRANSCRIPTION_MIN_SPEECH_MS: u32 = 2_000;
pub const DEFAULT_AI_TRANSCRIPTION_MERGE_GAP_MS: u32 = 700;
pub const MIN_AI_TRANSCRIPTION_MERGE_GAP_MS: u32 = 0;
pub const MAX_AI_TRANSCRIPTION_MERGE_GAP_MS: u32 = 5_000;
pub const DEFAULT_AI_TRANSCRIPTION_MAX_INITIAL_TIMESTAMP_SECONDS: f64 = 3.0;
pub const MIN_AI_TRANSCRIPTION_MAX_INITIAL_TIMESTAMP_SECONDS: f64 = 1.0;
pub const MAX_AI_TRANSCRIPTION_MAX_INITIAL_TIMESTAMP_SECONDS: f64 = 10.0;

pub fn available_cpu_core_count() -> u32 {
    std::thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1)
        .max(1) as u32
}

pub fn default_visual_frame_parallel_jobs() -> u32 {
    available_cpu_core_count().div_ceil(4).max(1)
}

pub fn default_settings() -> AppSettings {
    AppSettings {
        ai: default_ai_settings(),
        debug: DebugSettings { enabled: false },
        expert: ExpertSettings { enabled: false },
        record: RecordSettings {
            fps: DEFAULT_RECORD_FPS,
            video_bitrate_kbps: DEFAULT_VIDEO_BITRATE_KBPS,
            window_protection: default_window_protection_settings(),
        },
        storage: StorageSettings {
            retention_days: DEFAULT_STORAGE_RETENTION_DAYS,
            size_limit_bytes: DEFAULT_STORAGE_SIZE_LIMIT_BYTES,
        },
    }
}

pub fn default_window_protection_settings() -> RecordWindowProtectionSettings {
    RecordWindowProtectionSettings {
        default_action: ProtectedWindowAction::Pause,
        dwell_ms: DEFAULT_WINDOW_PROTECTION_DWELL_MS,
        enabled: false,
        merge_gap_ms: DEFAULT_WINDOW_PROTECTION_MERGE_GAP_MS,
        rules: Vec::new(),
    }
}

pub fn default_ai_settings() -> AiSettings {
    AiSettings {
        activity: AiActivitySettings {
            frame_deltas: AiActivitySourceSettings { enabled: true },
            keyboard: AiActivityWindowSettings {
                enabled: false,
                window_ms: DEFAULT_AI_KEYBOARD_WINDOW_MS,
            },
            mouse: AiActivityWindowSettings {
                enabled: false,
                window_ms: DEFAULT_AI_MOUSE_WINDOW_MS,
            },
            windows: AiActivitySourceSettings { enabled: true },
        },
        ocr: AiOcrSettings {
            enabled: true,
            engine: "tesseract".to_string(),
        },
        processing: AiProcessingSettings {
            auto_start_after_recording: true,
            frame_recognition_fps: DEFAULT_AI_FRAME_RECOGNITION_FPS,
            frame_recognition_sync_with_recording: true,
            frame_similarity_enabled: true,
            max_full_frame_checkpoints_per_minute: DEFAULT_AI_MAX_FULL_FRAME_CHECKPOINTS_PER_MINUTE,
            max_parallel_chunks_per_job: DEFAULT_AI_MAX_PARALLEL_CHUNKS_PER_JOB,
            max_parallel_ocr_jobs: default_visual_frame_parallel_jobs(),
            max_parallel_jobs: DEFAULT_AI_MAX_PARALLEL_JOBS,
            max_parallel_vlm_jobs: default_visual_frame_parallel_jobs(),
            periodic_full_frame_interval_ms: DEFAULT_AI_PERIODIC_FULL_FRAME_INTERVAL_MS,
            visual_delta_tile_size_px: DEFAULT_AI_VISUAL_DELTA_TILE_SIZE_PX,
        },
        transcription: AiTranscriptionProviderSettings {
            language: DEFAULT_AI_TRANSCRIPTION_LANGUAGE.to_string(),
            local_backend: DEFAULT_AI_TRANSCRIPTION_BACKEND.to_string(),
            local_model_id: DEFAULT_AI_TRANSCRIPTION_MODEL_ID.to_string(),
            max_initial_timestamp_seconds: DEFAULT_AI_TRANSCRIPTION_MAX_INITIAL_TIMESTAMP_SECONDS,
            merge_gap_ms: DEFAULT_AI_TRANSCRIPTION_MERGE_GAP_MS,
            min_speech_ms: DEFAULT_AI_TRANSCRIPTION_MIN_SPEECH_MS,
            provider: "local-whisper-cpp".to_string(),
            remote_api_key_ref: None,
            remote_base_url: None,
            remote_model: None,
            silence_gate_enabled: true,
            silence_threshold_db: DEFAULT_AI_TRANSCRIPTION_SILENCE_THRESHOLD_DB,
            word_timestamps_enabled: true,
        },
        vision: AiVisionProviderSettings {
            observations_enabled: false,
            api_key_ref: None,
            base_url: None,
            model: None,
            provider: "disabled".to_string(),
        },
    }
}

pub fn settings_limits() -> AppSettingsLimits {
    AppSettingsLimits {
        ai: AiSettingsLimits {
            activity: AiActivityLimits {
                keyboard_window_ms_min: MIN_AI_ACTIVITY_WINDOW_MS,
                keyboard_window_ms_max: MAX_AI_ACTIVITY_WINDOW_MS,
                mouse_window_ms_min: MIN_AI_ACTIVITY_WINDOW_MS,
                mouse_window_ms_max: MAX_AI_ACTIVITY_WINDOW_MS,
            },
            processing: AiProcessingLimits {
                frame_recognition_fps_min: MIN_AI_FRAME_RECOGNITION_FPS,
                frame_recognition_fps_max: MAX_AI_FRAME_RECOGNITION_FPS,
                max_full_frame_checkpoints_per_minute_min:
                    MIN_AI_MAX_FULL_FRAME_CHECKPOINTS_PER_MINUTE,
                max_full_frame_checkpoints_per_minute_max:
                    MAX_AI_MAX_FULL_FRAME_CHECKPOINTS_PER_MINUTE,
                max_parallel_chunks_per_job_min: MIN_AI_MAX_PARALLEL_CHUNKS_PER_JOB,
                max_parallel_chunks_per_job_max: MAX_AI_MAX_PARALLEL_CHUNKS_PER_JOB,
                max_parallel_jobs_min: MIN_AI_MAX_PARALLEL_JOBS,
                max_parallel_jobs_max: MAX_AI_MAX_PARALLEL_JOBS,
                max_parallel_ocr_jobs_min: MIN_AI_MAX_PARALLEL_OCR_JOBS,
                max_parallel_ocr_jobs_max: available_cpu_core_count(),
                max_parallel_vlm_jobs_min: MIN_AI_MAX_PARALLEL_VLM_JOBS,
                max_parallel_vlm_jobs_max: available_cpu_core_count(),
                periodic_full_frame_interval_ms_min: MIN_AI_PERIODIC_FULL_FRAME_INTERVAL_MS,
                periodic_full_frame_interval_ms_max: MAX_AI_PERIODIC_FULL_FRAME_INTERVAL_MS,
                visual_delta_tile_size_px_min: MIN_AI_VISUAL_DELTA_TILE_SIZE_PX,
                visual_delta_tile_size_px_max: MAX_AI_VISUAL_DELTA_TILE_SIZE_PX,
            },
            transcription: AiTranscriptionLimits {
                max_initial_timestamp_seconds_min:
                    MIN_AI_TRANSCRIPTION_MAX_INITIAL_TIMESTAMP_SECONDS,
                max_initial_timestamp_seconds_max:
                    MAX_AI_TRANSCRIPTION_MAX_INITIAL_TIMESTAMP_SECONDS,
                merge_gap_ms_min: MIN_AI_TRANSCRIPTION_MERGE_GAP_MS,
                merge_gap_ms_max: MAX_AI_TRANSCRIPTION_MERGE_GAP_MS,
                min_speech_ms_min: MIN_AI_TRANSCRIPTION_MIN_SPEECH_MS,
                min_speech_ms_max: MAX_AI_TRANSCRIPTION_MIN_SPEECH_MS,
                silence_threshold_db_min: MIN_AI_TRANSCRIPTION_SILENCE_THRESHOLD_DB,
                silence_threshold_db_max: MAX_AI_TRANSCRIPTION_SILENCE_THRESHOLD_DB,
            },
        },
        record: RecordSettingsLimits {
            fps_min: MIN_RECORD_FPS,
            fps_max: MAX_RECORD_FPS,
            window_protection_dwell_ms_min: MIN_WINDOW_PROTECTION_DWELL_MS,
            window_protection_dwell_ms_max: MAX_WINDOW_PROTECTION_DWELL_MS,
            window_protection_merge_gap_ms_min: MIN_WINDOW_PROTECTION_MERGE_GAP_MS,
            window_protection_merge_gap_ms_max: MAX_WINDOW_PROTECTION_MERGE_GAP_MS,
            window_protection_rule_count_max: MAX_WINDOW_PROTECTION_RULE_COUNT,
            window_protection_text_max_chars: MAX_WINDOW_PROTECTION_TEXT_CHARS,
            video_bitrate_kbps_min: MIN_VIDEO_BITRATE_KBPS,
            video_bitrate_kbps_max: MAX_VIDEO_BITRATE_KBPS,
        },
        storage: StorageSettingsLimits {
            retention_days_min: MIN_STORAGE_RETENTION_DAYS,
            retention_days_max: MAX_STORAGE_RETENTION_DAYS,
            size_limit_bytes_min: MIN_STORAGE_SIZE_LIMIT_BYTES,
            size_limit_bytes_max: None,
        },
    }
}
