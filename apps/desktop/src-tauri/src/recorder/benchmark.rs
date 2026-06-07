use std::{
    collections::BTreeMap,
    env,
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};

use serde::Serialize;
use tracing::warn;

use super::errors::filesystem_error;
use super::types::RecorderError;

const BENCHMARK_ENV: &str = "DAYDREAM_RECORDER_BENCHMARKS";
const SAMPLE_EVERY_ENV: &str = "DAYDREAM_RECORDER_BENCHMARK_SAMPLE_EVERY";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RecorderBenchmarkMode {
    Disabled,
    Jsonl,
    Summary,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RecorderBenchmarkConfig {
    mode: RecorderBenchmarkMode,
    sample_every: u64,
}

#[derive(Clone, Debug)]
pub(crate) struct RecorderBenchmarkSegmentInput {
    pub(crate) benchmarks_dir: PathBuf,
    pub(crate) canvas_height: u32,
    pub(crate) canvas_width: u32,
    pub(crate) desired_fps: f64,
    pub(crate) recording_id: String,
    pub(crate) segment_id: String,
    pub(crate) segment_index: u32,
}

#[derive(Clone, Debug)]
pub(crate) struct RecorderBenchmarkSession {
    inner: Arc<Mutex<RecorderBenchmarkInner>>,
}

#[derive(Debug)]
struct RecorderBenchmarkInner {
    config: RecorderBenchmarkConfig,
    context: RecorderBenchmarkContext,
    jsonl_file: Option<File>,
    next_event_index: u64,
    summary_path: PathBuf,
    stage_samples: BTreeMap<&'static str, Vec<u64>>,
}

#[derive(Clone, Debug)]
struct RecorderBenchmarkContext {
    canvas_height: u32,
    canvas_width: u32,
    desired_fps: f64,
    recording_id: String,
    segment_id: String,
    segment_index: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RecorderBenchmarkStage {
    Compose,
    Materialize,
    Passthrough,
    SendWait,
    Snapshot,
    WriterConvert,
    WriterPush,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RecorderBenchmarkStageEvent<'a> {
    schema_version: u32,
    kind: &'static str,
    recording_id: &'a str,
    segment_id: &'a str,
    segment_index: u32,
    event_index: u64,
    desired_fps: f64,
    timestamp_ms: Option<u64>,
    canvas: RecorderBenchmarkCanvas,
    source_count: Option<usize>,
    input_formats: Vec<String>,
    stage: &'static str,
    duration_ms: u64,
    bytes: Option<u64>,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RecorderBenchmarkCanvas {
    width: u32,
    height: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RecorderBenchmarkSummary<'a> {
    schema_version: u32,
    kind: &'static str,
    recording_id: &'a str,
    segment_id: &'a str,
    segment_index: u32,
    desired_fps: f64,
    canvas: RecorderBenchmarkCanvas,
    events: u64,
    stage_stats: BTreeMap<&'static str, RecorderBenchmarkStageSummary>,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RecorderBenchmarkStageSummary {
    avg_ms: f64,
    max_ms: u64,
    p50_ms: u64,
    p95_ms: u64,
    samples: u64,
}

impl RecorderBenchmarkConfig {
    pub(crate) fn from_env() -> Self {
        let mode = match env::var(BENCHMARK_ENV).ok().as_deref() {
            Some("jsonl") => RecorderBenchmarkMode::Jsonl,
            Some("summary") => RecorderBenchmarkMode::Summary,
            Some("0" | "false" | "off" | "disabled") | None => RecorderBenchmarkMode::Disabled,
            Some(value) => {
                warn!(
                    value,
                    env = BENCHMARK_ENV,
                    "recorder_benchmark_invalid_mode"
                );
                RecorderBenchmarkMode::Disabled
            }
        };
        let sample_every = env::var(SAMPLE_EVERY_ENV)
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(1);

        Self { mode, sample_every }
    }

    #[cfg(test)]
    pub(crate) fn jsonl_for_tests(sample_every: u64) -> Self {
        Self {
            mode: RecorderBenchmarkMode::Jsonl,
            sample_every: sample_every.max(1),
        }
    }

    #[cfg(test)]
    pub(crate) fn summary_for_tests() -> Self {
        Self {
            mode: RecorderBenchmarkMode::Summary,
            sample_every: 1,
        }
    }

    #[cfg(test)]
    pub(crate) fn disabled_for_tests() -> Self {
        Self {
            mode: RecorderBenchmarkMode::Disabled,
            sample_every: 1,
        }
    }

    fn enabled(&self) -> bool {
        self.mode != RecorderBenchmarkMode::Disabled
    }
}

impl RecorderBenchmarkSession {
    pub(crate) fn from_env(input: RecorderBenchmarkSegmentInput) -> Option<Self> {
        let config = RecorderBenchmarkConfig::from_env();
        if !config.enabled() {
            return None;
        }
        match Self::new(config, input) {
            Ok(session) => session,
            Err(error) => {
                warn!(
                    error_code = %error.code,
                    error_category = ?error.category,
                    "recorder_benchmark_start_failed"
                );
                None
            }
        }
    }

    pub(crate) fn new(
        config: RecorderBenchmarkConfig,
        input: RecorderBenchmarkSegmentInput,
    ) -> Result<Option<Self>, RecorderError> {
        if !config.enabled() {
            return Ok(None);
        }
        fs::create_dir_all(&input.benchmarks_dir).map_err(filesystem_error)?;
        let file_stem = format!("segment-{:04}", input.segment_index);
        let jsonl_path = input.benchmarks_dir.join(format!("{file_stem}.jsonl"));
        let summary_path = input
            .benchmarks_dir
            .join(format!("{file_stem}-summary.json"));
        let jsonl_file = match config.mode {
            RecorderBenchmarkMode::Jsonl => {
                Some(File::create(jsonl_path).map_err(filesystem_error)?)
            }
            RecorderBenchmarkMode::Summary | RecorderBenchmarkMode::Disabled => None,
        };

        Ok(Some(Self {
            inner: Arc::new(Mutex::new(RecorderBenchmarkInner {
                config,
                context: RecorderBenchmarkContext {
                    canvas_height: input.canvas_height,
                    canvas_width: input.canvas_width,
                    desired_fps: input.desired_fps,
                    recording_id: input.recording_id,
                    segment_id: input.segment_id,
                    segment_index: input.segment_index,
                },
                jsonl_file,
                next_event_index: 0,
                summary_path,
                stage_samples: BTreeMap::new(),
            })),
        }))
    }

    pub(crate) fn record_stage(
        &self,
        stage: RecorderBenchmarkStage,
        duration: Duration,
        timestamp_ms: Option<u64>,
        source_count: Option<usize>,
        input_formats: Vec<String>,
        bytes: Option<u64>,
    ) {
        let duration_ms = duration.as_millis().try_into().unwrap_or(u64::MAX);
        let Ok(mut inner) = self.inner.lock() else {
            return;
        };
        let event_index = inner.next_event_index;
        inner.next_event_index = inner.next_event_index.saturating_add(1);
        inner
            .stage_samples
            .entry(stage.label())
            .or_default()
            .push(duration_ms);

        if inner.config.mode != RecorderBenchmarkMode::Jsonl
            || event_index % inner.config.sample_every != 0
        {
            return;
        }

        let context = inner.context.clone();
        let event = RecorderBenchmarkStageEvent {
            schema_version: 1,
            kind: "stage",
            recording_id: &context.recording_id,
            segment_id: &context.segment_id,
            segment_index: context.segment_index,
            event_index,
            desired_fps: context.desired_fps,
            timestamp_ms,
            canvas: RecorderBenchmarkCanvas {
                width: context.canvas_width,
                height: context.canvas_height,
            },
            source_count,
            input_formats,
            stage: stage.label(),
            duration_ms,
            bytes,
        };
        if let Some(file) = inner.jsonl_file.as_mut() {
            if let Err(error) = write_json_line(file, &event) {
                warn!(%error, "recorder_benchmark_jsonl_write_failed");
            }
        }
    }

    pub(crate) fn finish(&self) {
        let Ok(inner) = self.inner.lock() else {
            return;
        };
        if !inner.config.enabled() {
            return;
        }
        let summary = inner.summary();
        if let Err(error) = write_summary(&inner.summary_path, &summary) {
            warn!(%error, "recorder_benchmark_summary_write_failed");
        }
    }
}

impl RecorderBenchmarkInner {
    fn summary(&self) -> RecorderBenchmarkSummary<'_> {
        RecorderBenchmarkSummary {
            schema_version: 1,
            kind: "segmentSummary",
            recording_id: &self.context.recording_id,
            segment_id: &self.context.segment_id,
            segment_index: self.context.segment_index,
            desired_fps: self.context.desired_fps,
            canvas: RecorderBenchmarkCanvas {
                width: self.context.canvas_width,
                height: self.context.canvas_height,
            },
            events: self.next_event_index,
            stage_stats: self
                .stage_samples
                .iter()
                .map(|(stage, samples)| (*stage, summarize_stage(samples)))
                .collect(),
        }
    }
}

impl RecorderBenchmarkStage {
    fn label(self) -> &'static str {
        match self {
            Self::Compose => "compose",
            Self::Materialize => "materialize",
            Self::Passthrough => "passthrough",
            Self::SendWait => "sendWait",
            Self::Snapshot => "snapshot",
            Self::WriterConvert => "writerConvert",
            Self::WriterPush => "writerPush",
        }
    }
}

fn summarize_stage(samples: &[u64]) -> RecorderBenchmarkStageSummary {
    let mut sorted = samples.to_vec();
    sorted.sort_unstable();
    let sum = sorted.iter().copied().sum::<u64>();
    let samples_len = sorted.len() as u64;

    RecorderBenchmarkStageSummary {
        avg_ms: if samples_len == 0 {
            0.0
        } else {
            sum as f64 / samples_len as f64
        },
        max_ms: sorted.last().copied().unwrap_or_default(),
        p50_ms: percentile(&sorted, 50),
        p95_ms: percentile(&sorted, 95),
        samples: samples_len,
    }
}

fn percentile(sorted: &[u64], percentile: usize) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    let index = ((sorted.len() - 1) * percentile) / 100;
    sorted[index]
}

fn write_json_line(file: &mut File, value: &impl Serialize) -> Result<(), std::io::Error> {
    serde_json::to_writer(&mut *file, value)?;
    file.write_all(b"\n")
}

fn write_summary(
    path: &Path,
    summary: &RecorderBenchmarkSummary<'_>,
) -> Result<(), std::io::Error> {
    let file = File::create(path)?;
    serde_json::to_writer_pretty(file, summary)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use uuid::Uuid;

    use super::*;

    #[test]
    fn benchmark_writer_creates_jsonl_and_summary_when_enabled() {
        let dir = std::env::temp_dir().join(format!("daydream-bench-{}", Uuid::now_v7()));
        let session = RecorderBenchmarkSession::new(
            RecorderBenchmarkConfig::jsonl_for_tests(1),
            benchmark_input(&dir),
        )
        .expect("benchmark session")
        .expect("enabled session");

        session.record_stage(
            RecorderBenchmarkStage::Materialize,
            Duration::from_millis(12),
            Some(1000),
            Some(1),
            vec!["I420".to_string()],
            Some(1024),
        );
        session.finish();

        let jsonl = fs::read_to_string(dir.join("segment-0001.jsonl")).expect("read jsonl");
        let summary =
            fs::read_to_string(dir.join("segment-0001-summary.json")).expect("read summary");

        assert!(jsonl.contains("\"kind\":\"stage\""));
        assert!(jsonl.contains("\"stage\":\"materialize\""));
        assert!(summary.contains("\"kind\": \"segmentSummary\""));
        assert!(summary.contains("\"materialize\""));

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn benchmark_writer_is_absent_when_disabled() {
        let dir = std::env::temp_dir().join(format!("daydream-bench-{}", Uuid::now_v7()));
        let session = RecorderBenchmarkSession::new(
            RecorderBenchmarkConfig::disabled_for_tests(),
            benchmark_input(&dir),
        )
        .expect("benchmark session");

        assert!(session.is_none());
        assert!(!dir.exists());
    }

    #[test]
    fn benchmark_writer_creates_only_summary_in_summary_mode() {
        let dir = std::env::temp_dir().join(format!("daydream-bench-{}", Uuid::now_v7()));
        let session = RecorderBenchmarkSession::new(
            RecorderBenchmarkConfig::summary_for_tests(),
            benchmark_input(&dir),
        )
        .expect("benchmark session")
        .expect("enabled session");

        session.record_stage(
            RecorderBenchmarkStage::WriterPush,
            Duration::from_millis(20),
            None,
            None,
            Vec::new(),
            None,
        );
        session.finish();

        assert!(!dir.join("segment-0001.jsonl").exists());
        assert!(dir.join("segment-0001-summary.json").exists());

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn benchmark_stage_summary_uses_sorted_percentiles() {
        let summary = summarize_stage(&[100, 1, 50, 20, 200]);

        assert_eq!(summary.samples, 5);
        assert_eq!(summary.p50_ms, 50);
        assert_eq!(summary.p95_ms, 100);
        assert_eq!(summary.max_ms, 200);
    }

    fn benchmark_input(dir: &Path) -> RecorderBenchmarkSegmentInput {
        RecorderBenchmarkSegmentInput {
            benchmarks_dir: dir.to_path_buf(),
            canvas_height: 1440,
            canvas_width: 5120,
            desired_fps: 2.0,
            recording_id: "recording-1".to_string(),
            segment_id: "segment-1".to_string(),
            segment_index: 1,
        }
    }
}
