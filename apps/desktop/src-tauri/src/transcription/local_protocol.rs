use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct LocalWarmupRequest {
    #[serde(rename = "daydreamLocalBackend")]
    pub(super) local_backend: String,
    pub(super) model: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WhisperWorkerReady {
    pub(super) token: String,
    pub(super) transcription_url: String,
    pub(super) warmup_url: String,
}
