use crate::storage::{
    self, AppStorage, RecorderSceneRow, RecorderSceneSourceRow,
    SaveRecorderSceneInput as StorageSaveRecorderSceneInput,
    SaveRecorderSceneSourceInput as StorageSaveRecorderSceneSourceInput,
};

use super::errors::{invalid_scene_error, storage_error, storage_error_message};
use super::types::{
    CanvasRect, CaptureSourceKind, CropRect, RecorderError, RecorderQualityPreset, RecorderScene,
    RecorderSceneLayoutMode, RecorderSceneSource, SaveRecorderSceneInput,
};

pub(crate) fn save_recorder_scene_for_storage(
    storage: &AppStorage,
    input: SaveRecorderSceneInput,
    timestamp_ms: u64,
) -> Result<RecorderScene, RecorderError> {
    let storage_input = storage_scene_input_from_recorder(input.scene, timestamp_ms)?;
    let scene = storage
        .with_connection_mut(|connection| storage::save_recorder_scene(connection, &storage_input))
        .map_err(storage_error)?;

    scene_from_storage(scene)
}

pub(crate) fn latest_recorder_scene_for_storage(
    storage: &AppStorage,
) -> Result<Option<RecorderScene>, RecorderError> {
    let scene = storage
        .with_connection(storage::get_latest_recorder_scene)
        .map_err(storage_error)?;

    scene.map(scene_from_storage).transpose()
}

pub(crate) fn quality_preset_as_storage_label(preset: &RecorderQualityPreset) -> &'static str {
    match preset {
        RecorderQualityPreset::BalancedOcr => "balanced-ocr",
    }
}

fn storage_scene_input_from_recorder(
    scene: RecorderScene,
    timestamp_ms: u64,
) -> Result<StorageSaveRecorderSceneInput, RecorderError> {
    Ok(StorageSaveRecorderSceneInput {
        canvas_height: scene.canvas_height.into(),
        canvas_width: scene.canvas_width.into(),
        desktop_audio_source_id: scene.desktop_audio_source_id,
        id: scene.id,
        layout_mode: layout_mode_as_storage_label(&scene.layout_mode).to_string(),
        microphone_audio_source_id: scene.microphone_audio_source_id,
        name: scene.name,
        sources: scene
            .sources
            .into_iter()
            .map(storage_scene_source_input_from_recorder)
            .collect::<Result<Vec<_>, _>>()?,
        timestamp_ms: timestamp_ms as i64,
    })
}

fn storage_scene_source_input_from_recorder(
    source: RecorderSceneSource,
) -> Result<StorageSaveRecorderSceneSourceInput, RecorderError> {
    Ok(StorageSaveRecorderSceneSourceInput {
        crop_bottom: pixel_value(source.crop.bottom, "crop.bottom")?,
        crop_left: pixel_value(source.crop.left, "crop.left")?,
        crop_right: pixel_value(source.crop.right, "crop.right")?,
        crop_top: pixel_value(source.crop.top, "crop.top")?,
        display_name: source.display_name,
        height: pixel_value(source.rect.height, "rect.height")?,
        id: source.id,
        preserve_aspect_ratio: source.preserve_aspect_ratio,
        source_id: source.source_id,
        source_height: optional_pixel_value(source.source_height, "sourceHeight")?,
        source_kind: capture_source_kind_as_storage_label(&source.source_kind).to_string(),
        source_width: optional_pixel_value(source.source_width, "sourceWidth")?,
        width: pixel_value(source.rect.width, "rect.width")?,
        x: pixel_value(source.rect.x, "rect.x")?,
        y: pixel_value(source.rect.y, "rect.y")?,
        z_index: source.z_index.into(),
    })
}

pub(crate) fn scene_from_storage(scene: RecorderSceneRow) -> Result<RecorderScene, RecorderError> {
    Ok(RecorderScene {
        canvas_height: u32_value(scene.canvas_height, "canvasHeight")?,
        canvas_width: u32_value(scene.canvas_width, "canvasWidth")?,
        desktop_audio_source_id: scene.desktop_audio_source_id,
        id: scene.id,
        layout_mode: layout_mode_from_storage_label(&scene.layout_mode)?,
        microphone_audio_source_id: scene.microphone_audio_source_id,
        name: scene.name,
        sources: scene
            .sources
            .into_iter()
            .map(scene_source_from_storage)
            .collect::<Result<Vec<_>, _>>()?,
    })
}

fn scene_source_from_storage(
    source: RecorderSceneSourceRow,
) -> Result<RecorderSceneSource, RecorderError> {
    Ok(RecorderSceneSource {
        crop: CropRect {
            bottom: source.crop_bottom as f64,
            left: source.crop_left as f64,
            right: source.crop_right as f64,
            top: source.crop_top as f64,
        },
        display_name: source.display_name,
        id: source.id,
        preserve_aspect_ratio: source.preserve_aspect_ratio,
        rect: CanvasRect {
            height: source.height as f64,
            width: source.width as f64,
            x: source.x as f64,
            y: source.y as f64,
        },
        source_id: source.source_id,
        source_height: source.source_height.map(|height| height as f64),
        source_kind: capture_source_kind_from_storage_label(&source.source_kind)?,
        source_width: source.source_width.map(|width| width as f64),
        z_index: source.z_index as i32,
    })
}

fn capture_source_kind_as_storage_label(kind: &CaptureSourceKind) -> &'static str {
    match kind {
        CaptureSourceKind::Monitor => "monitor",
    }
}

fn capture_source_kind_from_storage_label(value: &str) -> Result<CaptureSourceKind, RecorderError> {
    match value {
        "monitor" => Ok(CaptureSourceKind::Monitor),
        _ => Err(storage_error_message("Unknown capture source kind.")),
    }
}

fn layout_mode_as_storage_label(layout_mode: &RecorderSceneLayoutMode) -> &'static str {
    match layout_mode {
        RecorderSceneLayoutMode::Auto169 => "auto-16-9",
        RecorderSceneLayoutMode::Custom => "custom",
        RecorderSceneLayoutMode::Horizontal => "horizontal",
        RecorderSceneLayoutMode::Vertical => "vertical",
    }
}

fn layout_mode_from_storage_label(value: &str) -> Result<RecorderSceneLayoutMode, RecorderError> {
    match value {
        "auto-16-9" => Ok(RecorderSceneLayoutMode::Auto169),
        "custom" => Ok(RecorderSceneLayoutMode::Custom),
        "horizontal" => Ok(RecorderSceneLayoutMode::Horizontal),
        "vertical" => Ok(RecorderSceneLayoutMode::Vertical),
        _ => Err(storage_error_message("Unknown scene layout mode.")),
    }
}

fn pixel_value(value: f64, field: &str) -> Result<i64, RecorderError> {
    if !value.is_finite() {
        return Err(invalid_scene_error(field));
    }

    Ok(value.round() as i64)
}

fn optional_pixel_value(value: Option<f64>, field: &str) -> Result<Option<i64>, RecorderError> {
    value.map(|value| pixel_value(value, field)).transpose()
}

fn u32_value(value: i64, field: &str) -> Result<u32, RecorderError> {
    value
        .try_into()
        .map_err(|_error| invalid_scene_error(field))
}
