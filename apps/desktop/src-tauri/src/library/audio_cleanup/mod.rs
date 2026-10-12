//! Reversible local microphone cleanup. Captured files are never rewritten.
use serde::{Deserialize, Serialize};

pub(crate) mod commands;
mod dsp;
pub(crate) mod media;
mod persistence;
mod preview;
mod runtime;
mod service;

pub(crate) use persistence::privacy_fingerprint;
pub(crate) use persistence::{active_manifest, pin_job, pinned_manifest, recover_interrupted};
pub(crate) use runtime::request_cancel;
pub(crate) use service::{cancel_recording, read_state};

pub(super) const ENGINE_VERSION: &str = "webrtc-apm-2.1.0-cleanup-v3";
pub(super) const SAMPLE_RATE: usize = 48_000;
pub(super) const BLOCK_MS: i64 = 30_000;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct AudioCleanupConfig {
    pub mode: CleanupMode,
    pub match_threshold: f64,
    pub amount: f64,
}
impl Default for AudioCleanupConfig {
    fn default() -> Self {
        Self {
            mode: CleanupMode::Off,
            match_threshold: 0.65,
            amount: 1.0,
        }
    }
}
impl AudioCleanupConfig {
    pub(super) fn validate(&self) -> Result<(), String> {
        if !self.match_threshold.is_finite()
            || !(0.1..=0.99).contains(&self.match_threshold)
            || !self.amount.is_finite()
            || !(0.0..=1.0).contains(&self.amount)
        {
            return Err(
                "Choose a match threshold from 0.1 to 0.99 and cancellation from 0 to 100%.".into(),
            );
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum CleanupMode {
    Off,
    Auto,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AudioCleanupState {
    pub recording_id: String,
    pub config: AudioCleanupConfig,
    pub pending_config: Option<AudioCleanupConfig>,
    pub revision: Option<String>,
    pub phase: String,
    pub progress: f64,
    pub matched_ms: i64,
    pub reference_available: bool,
    pub transcripts_outdated: bool,
    pub error: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AudioCleanupPreview {
    pub recording_id: String,
    pub session_id: String,
    pub original_url: String,
    pub cleaned_url: String,
    pub matched_ms: i64,
    pub duration_ms: i64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ApplyAudioCleanupInput {
    pub recording_id: String,
    pub config: AudioCleanupConfig,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PreviewAudioCleanupInput {
    pub recording_id: String,
    pub config: AudioCleanupConfig,
    pub start_offset_ms: i64,
    pub end_offset_ms: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct CleanupBlock {
    pub segment_id: String,
    pub start_ms: i64,
    pub duration_ms: i64,
    pub path: String,
    pub matched_ms: i64,
}

#[derive(Clone, Debug)]
pub(crate) struct CleanupManifest {
    pub revision: String,
    pub blocks: Vec<CleanupBlock>,
}

#[cfg(test)]
mod integration_tests;
#[cfg(test)]
mod tests;
