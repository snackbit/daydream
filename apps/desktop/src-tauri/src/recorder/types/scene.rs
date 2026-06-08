use serde::{Deserialize, Serialize};

use super::{CanvasRect, CaptureSourceKind, CropRect};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum RecorderSceneLayoutMode {
    #[serde(rename = "auto-16-9")]
    Auto169,
    Custom,
    Horizontal,
    Vertical,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecorderSceneSource {
    pub(crate) crop: CropRect,
    pub(crate) display_name: String,
    pub(crate) id: String,
    pub(crate) preserve_aspect_ratio: bool,
    pub(crate) rect: CanvasRect,
    pub(crate) source_id: String,
    pub(crate) source_height: Option<f64>,
    pub(crate) source_kind: CaptureSourceKind,
    pub(crate) source_width: Option<f64>,
    pub(crate) z_index: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecorderScene {
    pub(crate) canvas_height: u32,
    pub(crate) canvas_width: u32,
    pub(crate) desktop_audio_source_id: Option<String>,
    pub(crate) id: String,
    pub(crate) layout_mode: RecorderSceneLayoutMode,
    pub(crate) microphone_audio_source_id: Option<String>,
    pub(crate) name: String,
    pub(crate) sources: Vec<RecorderSceneSource>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SaveRecorderSceneInput {
    pub(crate) scene: RecorderScene,
}
