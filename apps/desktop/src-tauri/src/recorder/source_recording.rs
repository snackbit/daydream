//! Translate reviewed source settings into a frozen recording snapshot.
use super::{
    errors::invalid_scene_error,
    types::{RecorderError, RecorderScene},
};
use crate::storage::RecordingVideoSource;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SaveCaptureSetupInput {
    pub(crate) scene: RecorderScene,
    pub(crate) fps: f64,
    pub(crate) video_bitrate_kbps: u32,
}
#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CaptureSetup {
    scene: Option<RecorderScene>,
    fps: f64,
    video_bitrate_kbps: u32,
}

#[tauri::command(rename = "getCaptureSetup")]
pub(crate) fn get_capture_setup(
    storage: tauri::State<'_, crate::storage::AppStorage>,
) -> Result<CaptureSetup, RecorderError> {
    let settings = crate::settings::load_app_settings(&storage).map_err(|_| {
        super::errors::storage_error_message("Recording settings could not be loaded.")
    })?;
    Ok(CaptureSetup {
        scene: super::storage_mapping::latest_recorder_scene_for_storage(&storage)?,
        fps: settings.record.fps,
        video_bitrate_kbps: settings.record.video_bitrate_kbps,
    })
}

#[tauri::command(rename = "saveCaptureSetup")]
pub(crate) fn save_capture_setup(
    input: SaveCaptureSetupInput,
    app: tauri::AppHandle,
    storage: tauri::State<'_, crate::storage::AppStorage>,
    manager: tauri::State<'_, super::manager::RecorderManager>,
) -> Result<CaptureSetup, RecorderError> {
    let _activity =
        crate::maintenance::runtime::admit(&app, "capture-configure").map_err(|message| {
            super::errors::lifecycle_error("recorder_maintenance_active", &message)
        })?;
    if manager.configuration_is_locked()? {
        return Err(super::errors::lifecycle_error(
            "recorder_configuration_locked",
            "Stop recording before changing its sources.",
        ));
    }
    persist_capture_setup(input, &storage)
}

pub(crate) fn persist_capture_setup(
    input: SaveCaptureSetupInput,
    storage: &crate::storage::AppStorage,
) -> Result<CaptureSetup, RecorderError> {
    let mut settings = crate::settings::load_app_settings(storage).map_err(|_| {
        super::errors::storage_error_message("Recording settings could not be loaded.")
    })?;
    settings.record.fps = input.fps;
    settings.record.video_bitrate_kbps = input.video_bitrate_kbps;
    crate::settings::validate_settings(settings)
        .map_err(|error| super::errors::storage_error_message(&error.message))?;
    let scene = input.scene;
    if !scene.independent_video {
        return Err(invalid_scene_error(
            "Select and review at least one video source.",
        ));
    }
    video_sources(&scene)?;
    let timestamp = super::manager::now_ms();
    let row = super::storage_mapping::storage_scene_input_from_recorder(scene, timestamp)?;
    let saved = storage
        .with_connection_mut(|connection| {
            let transaction = connection.transaction()?;
            crate::storage::save_recorder_scene_rows(&transaction, &row, true)?;
            for (key, value) in [
                ("record.fps", serde_json::json!(input.fps)),
                (
                    "record.videoBitrateKbps",
                    serde_json::json!(input.video_bitrate_kbps),
                ),
            ] {
                crate::storage::save_app_setting(
                    &transaction,
                    &crate::storage::SaveAppSettingInput {
                        key: key.into(),
                        value_json: value.to_string(),
                        is_sensitive: false,
                        timestamp_ms: timestamp as i64,
                    },
                )?;
            }
            transaction.commit()?;
            crate::storage::get_recorder_scene(connection, &row.id)
        })
        .map_err(super::errors::storage_error)?;
    Ok(CaptureSetup {
        scene: Some(super::storage_mapping::scene_from_storage(saved)?),
        fps: input.fps,
        video_bitrate_kbps: input.video_bitrate_kbps,
    })
}

pub(crate) fn video_sources(
    scene: &RecorderScene,
) -> Result<Vec<RecordingVideoSource>, RecorderError> {
    if !scene.independent_video {
        return Ok(Vec::new());
    }
    scene
        .sources
        .iter()
        .map(|source| {
            let (Some(width), Some(height)) = (source.source_width, source.source_height) else {
                return Err(invalid_scene_error(
                    "Source dimensions must be reviewed before saving.",
                ));
            };
            let crop_width = width - source.crop.left - source.crop.right;
            let crop_height = height - source.crop.top - source.crop.bottom;
            if [
                width,
                height,
                source.crop.left,
                source.crop.right,
                source.crop.top,
                source.crop.bottom,
            ]
            .iter()
            .any(|value| !value.is_finite() || value.fract() != 0.0 || *value < 0.0)
                || crop_width < 2.0
                || crop_height < 2.0
                || source.rect.width > crop_width
                || source.rect.height > crop_height
                || source.rect.width % 2.0 != 0.0
                || source.rect.height % 2.0 != 0.0
            {
                return Err(invalid_scene_error(
                    "Crop and output dimensions must stay inside their source.",
                ));
            }
            if !source.rect.width.is_finite()
                || !source.rect.height.is_finite()
                || source.rect.width < 2.0
                || source.rect.height < 2.0
            {
                return Err(invalid_scene_error(
                    "Each source needs valid output dimensions.",
                ));
            }
            Ok(RecordingVideoSource {
                track_id: uuid::Uuid::now_v7().to_string(),
                source_id: source.id.clone(),
                label: source.display_name.clone(),
                width: (source.rect.width as u32) / 2 * 2,
                height: (source.rect.height as u32) / 2 * 2,
                config_json: serde_json::to_string(source)
                    .map_err(|_| invalid_scene_error("Source settings could not be saved."))?,
            })
        })
        .collect()
}
