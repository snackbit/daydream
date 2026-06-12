use std::{
    path::PathBuf,
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug)]
pub struct TranscriptionRequest {
    pub audio_path: PathBuf,
    pub cancel: Arc<AtomicBool>,
    pub language: String,
    pub max_initial_timestamp_seconds: f64,
    pub model: String,
    pub timeout: Duration,
    pub word_timestamps: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VerboseTranscription {
    pub duration: Option<f64>,
    pub language: Option<String>,
    pub segments: Vec<VerboseTranscriptionSegment>,
    pub task: Option<String>,
    pub text: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub words: Vec<VerboseTranscriptionWord>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VerboseTranscriptionSegment {
    pub end: f64,
    pub id: i64,
    pub start: f64,
    pub text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VerboseTranscriptionWord {
    pub end: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub probability: Option<f64>,
    pub start: f64,
    pub word: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptionError {
    pub code: String,
    pub message: String,
}

#[derive(Clone, Debug)]
pub struct LocalTranscriptionInput {
    pub audio_path: PathBuf,
    pub cancel: Arc<AtomicBool>,
    pub language: String,
    pub local_backend: String,
    pub max_initial_timestamp_seconds: f64,
    pub model_id: String,
    pub model_path: PathBuf,
    pub word_timestamps: bool,
}

#[derive(Clone, Debug)]
pub struct LocalTranscriptionWarmup {
    pub local_backend: String,
    pub model_id: String,
    pub model_path: PathBuf,
}
