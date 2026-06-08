use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum CaptureSourceKind {
    Monitor,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CaptureSource {
    pub(crate) display_name: String,
    pub(crate) height: Option<u32>,
    pub(crate) id: String,
    pub(crate) kind: CaptureSourceKind,
    pub(crate) logical_height: Option<u32>,
    pub(crate) logical_width: Option<u32>,
    pub(crate) scale_factor: Option<f64>,
    pub(crate) width: Option<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CaptureSourceList {
    pub(crate) sources: Vec<CaptureSource>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CaptureSourceSelection {
    pub(crate) source_ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SelectCaptureSourcesInput {
    #[serde(default)]
    pub(crate) force_chooser: bool,
}
