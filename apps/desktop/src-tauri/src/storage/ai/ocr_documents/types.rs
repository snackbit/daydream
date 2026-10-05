use crate::{ai::VisualRegion, storage::CreateVisualRegionInput};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OcrWordHierarchy {
    pub page_num: i64,
    pub block_num: i64,
    pub paragraph_num: i64,
    pub line_num: i64,
    pub word_num: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OcrWordInput {
    pub id: String,
    pub hierarchy: Option<OcrWordHierarchy>,
    pub text: String,
    pub confidence: Option<f64>,
    pub region: Option<VisualRegion>,
    pub raw_region_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OcrBlockInput {
    pub id: String,
    pub order: i64,
    pub text: String,
    pub confidence: Option<f64>,
    pub region: Option<VisualRegion>,
    pub raw_region_ids: Vec<String>,
    pub words: Vec<OcrWordInput>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OcrContentInput {
    pub text: String,
    pub blocks: Vec<OcrBlockInput>,
}

#[derive(Clone, Debug)]
pub struct OcrCachedContent {
    pub content_id: String,
    pub content: OcrContentInput,
}

#[derive(Clone, Debug)]
pub struct OcrFrameInput {
    pub id: String,
    pub frame_timestamp_ms: i64,
    pub sample_end_ms: i64,
    pub source_frame_ref: String,
    pub source_track_id: String,
    pub width: i64,
    pub height: i64,
    pub config_fingerprint: String,
    pub engine: String,
    pub provider: String,
    pub model: Option<String>,
    pub pixel_hash: String,
    pub status: String,
    pub error_code: Option<String>,
    pub content: Option<OcrContentInput>,
    pub cache_content_id: Option<String>,
    pub raw_regions: Vec<CreateVisualRegionInput>,
}
