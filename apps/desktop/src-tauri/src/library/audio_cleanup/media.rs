//! Bounded decoding/encoding at the FFmpeg boundary, with private temporary files.
use super::*;
use crate::{
    ffmpeg,
    storage::{RecordingSegmentRow, RedactionRow},
};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::AtomicBool,
    time::Duration,
};

pub(super) struct TemporaryFile(pub PathBuf);
impl Drop for TemporaryFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}
pub(super) fn private_file(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|_| "Cleanup storage could not be prepared.")?;
    }
    let mut options = fs::OpenOptions::new();
    options.create(true).truncate(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options
        .open(path)
        .map_err(|_| "Cleanup storage could not be prepared.")?;
    Ok(())
}
fn run(command: &mut Command, cancel: &AtomicBool) -> Result<(), String> {
    let output = crate::subprocess::run(command, cancel, Duration::from_secs(60), 64_000)
        .map_err(|_| "Audio preparation was cancelled or did not finish.")?;
    if !output.status.success() {
        return Err("Local audio preparation failed.".into());
    }
    Ok(())
}

pub(super) fn decode(
    path: &str,
    stream: i64,
    start: i64,
    duration: i64,
    temp_dir: &Path,
    cancel: &AtomicBool,
) -> Result<(Vec<f32>, usize), String> {
    decode_at_rate(
        path,
        stream,
        start,
        duration,
        temp_dir,
        cancel,
        SAMPLE_RATE as u32,
    )
}

fn decode_at_rate(
    path: &str,
    stream: i64,
    start: i64,
    duration: i64,
    temp_dir: &Path,
    cancel: &AtomicBool,
    sample_rate: u32,
) -> Result<(Vec<f32>, usize), String> {
    if start < 0 || duration <= 0 || duration > BLOCK_MS + 2000 {
        return Err("Audio range is invalid.".into());
    }
    let file = TemporaryFile(temp_dir.join(format!("decode-{}.wav", uuid::Uuid::now_v7())));
    private_file(&file.0)?;
    run(
        Command::new(ffmpeg::binary_path())
            .args([
                "-hide_banner",
                "-v",
                "error",
                "-threads",
                "2",
                "-y",
                "-i",
                path,
                "-ss",
                &format!("{:.3}", start as f64 / 1000.0),
                "-t",
                &format!("{:.3}", duration as f64 / 1000.0),
                "-map",
                &format!("0:{stream}"),
                "-vn",
                "-af",
                &ffmpeg::audio_timeline_filter(sample_rate, start + duration),
                "-ar",
                &sample_rate.to_string(),
                "-c:a",
                "pcm_f32le",
                "-f",
                "wav",
            ])
            .arg(&file.0),
        cancel,
    )?;
    let mut reader =
        hound::WavReader::open(&file.0).map_err(|_| "Decoded audio could not be read.")?;
    let channels = reader.spec().channels as usize;
    if !(1..=8).contains(&channels) {
        return Err("Audio channel layout is unsupported.".into());
    }
    let count = duration as usize * (sample_rate as usize / 1000) * channels;
    let mut samples = reader
        .samples::<f32>()
        .take(count)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "Decoded audio is invalid.")?;
    samples.resize(count, 0.0);
    Ok((samples, channels))
}

pub(super) fn encode(
    samples: &[f32],
    channels: usize,
    path: &Path,
    cancel: &AtomicBool,
) -> Result<(), String> {
    let wav = TemporaryFile(path.with_extension("input.wav"));
    let staged = TemporaryFile(path.with_extension("pending.flac"));
    private_file(&wav.0)?;
    private_file(&staged.0)?;
    let spec = hound::WavSpec {
        channels: channels as u16,
        sample_rate: SAMPLE_RATE as u32,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut writer = hound::WavWriter::create(&wav.0, spec)
        .map_err(|_| "Cleaned audio could not be written.")?;
    for &sample in samples {
        writer
            .write_sample(sample)
            .map_err(|_| "Cleaned audio could not be written.")?;
    }
    writer
        .finalize()
        .map_err(|_| "Cleaned audio could not be finalized.")?;
    let mut command = Command::new(ffmpeg::binary_path());
    command.args(["-hide_banner", "-v", "error", "-y"]);
    // Hound's extensible WAV header labels one channel as front-left. A single
    // microphone channel is mono; reinterpret its layout without remixing it.
    if channels == 1 {
        command.args(["-channel_layout", "mono"]);
    }
    command
        .arg("-i")
        .arg(&wav.0)
        .args([
            "-c:a",
            "flac",
            "-sample_fmt",
            "s32",
            "-threads",
            "2",
            "-f",
            "flac",
        ])
        .arg(&staged.0);
    run(&mut command, cancel)?;
    fs::rename(&staged.0, path).map_err(|_| "Cleaned audio could not be published.".into())
}

/// Partitions include microphone and reference cuts. Hidden microphone is silent;
/// a hidden reference leaves eligible microphone unchanged and resets adaptation.
pub(super) fn spans(
    segment: &RecordingSegmentRow,
    base: i64,
    start: i64,
    end: i64,
    redactions: &[RedactionRow],
) -> Vec<(i64, i64, bool, bool)> {
    let mic = crate::recording_privacy::source_track_id(segment, "microphone");
    let desktop = crate::recording_privacy::source_track_id(segment, "desktop-audio");
    let mut edges = vec![start, end];
    for cut in redactions {
        for edge in [cut.start_offset_ms - base, cut.end_offset_ms - base] {
            if start < edge && edge < end {
                edges.push(edge);
            }
        }
    }
    edges.sort_unstable();
    edges.dedup();
    edges
        .windows(2)
        .map(|edges| {
            let (a, b) = (edges[0], edges[1]);
            let eligible = |id: &Option<String>| {
                id.as_ref().is_some_and(|id| {
                    !crate::recording_privacy::range_redacted(
                        redactions,
                        &[
                            id.clone(),
                            if id.starts_with("microphone-") {
                                "microphone".into()
                            } else {
                                "desktop-audio".into()
                            },
                        ],
                        a + base,
                        b + base,
                    )
                })
            };
            (a, b, eligible(&mic), eligible(&desktop))
        })
        .collect()
}

pub(super) struct AudioBlockInput<'a> {
    pub segment: &'a RecordingSegmentRow,
    pub base: i64,
    pub start: i64,
    pub duration: i64,
}
pub(super) fn render_block(
    input: AudioBlockInput<'_>,
    redactions: &[RedactionRow],
    config: &AudioCleanupConfig,
    dir: &Path,
    cancel: &AtomicBool,
) -> Result<(Vec<f32>, Vec<f32>, usize, i64), String> {
    let AudioBlockInput {
        segment,
        base,
        start,
        duration,
    } = input;
    let stream = segment
        .microphone_audio_stream_index
        .ok_or("Microphone audio is missing.")?;
    let from = start.saturating_sub(2000).max(0);
    let end = start + duration;
    let mut channels = 0;
    let mut original = Vec::new();
    let mut cleaned = Vec::new();
    let mut matched = 0;
    for (a, b, mic, desktop) in spans(segment, base, from, end, redactions) {
        if !mic {
            continue;
        }
        let (samples, count) = decode(&segment.file_path, stream, a, b - a, dir, cancel)?;
        if channels == 0 {
            channels = count;
            original.resize(duration as usize * 48 * channels, 0.0);
            cleaned.resize(original.len(), 0.0);
        }
        if channels != count {
            return Err("Microphone channel layout changed.".into());
        }
        let result = if desktop && config.mode == CleanupMode::Auto && config.amount > 0.0 {
            let (reference, reference_channels) = decode(
                &segment.file_path,
                segment
                    .desktop_audio_stream_index
                    .ok_or("Desktop audio is missing.")?,
                a,
                b - a,
                dir,
                cancel,
            )?;
            dsp::clean(
                &samples,
                channels,
                &reference,
                reference_channels,
                config,
                cancel,
            )?
        } else {
            dsp::CleanedAudio {
                samples: samples.clone(),
                matched_ms: 0,
            }
        };
        let owned_from = a.max(start);
        if owned_from >= b {
            continue;
        }
        let input_from = (owned_from - a) as usize * 48 * channels;
        let output_from = (owned_from - start) as usize * 48 * channels;
        let count = (b - owned_from) as usize * 48 * channels;
        original[output_from..output_from + count]
            .copy_from_slice(&samples[input_from..input_from + count]);
        cleaned[output_from..output_from + count]
            .copy_from_slice(&result.samples[input_from..input_from + count]);
        matched += result.matched_ms.min(b - owned_from);
    }
    if channels == 0 {
        // No eligible media is decoded merely to discover a hidden source's layout.
        channels=segment.probe_json.as_deref().and_then(|json|serde_json::from_str::<serde_json::Value>(json).ok()).and_then(|value|value.get("audio")?.as_array()?.iter().find(|audio|audio.get("index").and_then(serde_json::Value::as_i64)==Some(stream)).and_then(|audio|audio.get("channels").and_then(serde_json::Value::as_u64))).and_then(|channels|usize::try_from(channels).ok()).filter(|channels|(1..=8).contains(channels)).ok_or("Microphone channel metadata is missing. Recover the recording metadata before cleanup.")?;
        original.resize(duration as usize * 48 * channels, 0.0);
        cleaned.resize(original.len(), 0.0);
    }
    Ok((original, cleaned, channels, matched))
}

pub(crate) fn extract_cleaned_window(
    manifest: &CleanupManifest,
    segment: &RecordingSegmentRow,
    start: i64,
    duration: i64,
    output: &Path,
    cancel: &AtomicBool,
) -> Result<(), String> {
    let dir = output.parent().ok_or("Audio output is unavailable.")?;
    let mut samples = Vec::new();
    let mut cursor = start;
    for block in persistence::segment_blocks(manifest, segment) {
        let from = cursor.max(block.start_ms);
        let to = (start + duration).min(block.start_ms + block.duration_ms);
        if from >= to {
            continue;
        }
        if from != cursor {
            return Err("Cleaned audio is incomplete. Retry cleanup.".into());
        }
        let (part, channels) = decode_at_rate(
            &block.path,
            0,
            from - block.start_ms,
            to - from,
            dir,
            cancel,
            16_000,
        )?;
        samples.extend(
            part.chunks_exact(channels)
                .map(|frame| frame.iter().sum::<f32>() / channels as f32),
        );
        cursor = to;
    }
    if cursor != start + duration {
        return Err("Cleaned audio is incomplete. Retry cleanup.".into());
    }
    crate::whisper_backend::vad::write_detection_samples(output, &samples)
        .map_err(|_| "Cleaned transcription input could not be written.".into())
}
