use serde::{Deserialize, Serialize};

use super::RecorderError;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StartRecorderPreviewInput {
    pub(crate) scene: Option<super::RecorderScene>,
    pub(crate) scene_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum RecorderPreviewState {
    Error,
    Idle,
    Live,
    Starting,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecorderPreviewStatus {
    pub(crate) error: Option<RecorderError>,
    pub(crate) scene_id: Option<String>,
    pub(crate) source_ids: Vec<String>,
    pub(crate) state: RecorderPreviewState,
    pub(crate) updated_at_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecorderPreviewFrame {
    pub(crate) source_width: u32,
    pub(crate) source_height: u32,
    pub(crate) data_url: String,
    pub(crate) height: u32,
    pub(crate) scene_source_id: String,
    pub(crate) source_id: String,
    pub(crate) timestamp_ms: u64,
    pub(crate) width: u32,
}
