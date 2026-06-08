use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CanvasRect {
    pub(crate) height: f64,
    pub(crate) width: f64,
    pub(crate) x: f64,
    pub(crate) y: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CropRect {
    pub(crate) bottom: f64,
    pub(crate) left: f64,
    pub(crate) right: f64,
    pub(crate) top: f64,
}
