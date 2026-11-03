//! Raw peaks are prepared once. Viewport reads only aggregate persisted bins.
use super::{
    cache::{waveform_file_fingerprint, waveform_tile_cache_path},
    math::waveform_peak_rms_time_bins,
};
use crate::storage::{RecordingRow, RecordingSegmentRow};
use std::{
    fs::{self, File},
    io::{BufWriter, Read, Seek, SeekFrom, Write},
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

const MAGIC: &[u8; 8] = b"DWFM0001";
const HEADER_BYTES: u64 = 40;
pub(super) const BASE_BIN_MS: i64 = 5;

pub(crate) struct PreparedSource {
    pub segment: RecordingSegmentRow,
    pub track_id: String,
    pub source_kind: String,
    pub stream: i64,
    pub offset_ms: i64,
    pub start_ms: i64,
    pub end_ms: i64,
    pub path: PathBuf,
}
impl PreparedSource {
    pub(super) fn new(
        recording: &RecordingRow,
        segment: RecordingSegmentRow,
        track_id: String,
        source_kind: String,
        stream: i64,
        offset_ms: i64,
        range: std::ops::Range<i64>,
    ) -> Result<Self, String> {
        let start_ms = range.start;
        let end_ms = range.end;
        let fingerprint = waveform_file_fingerprint(&segment.file_path)?;
        let path = waveform_tile_cache_path(
            recording,
            &segment,
            stream,
            BASE_BIN_MS,
            start_ms - offset_ms,
            &fingerprint,
        )
        .with_extension(format!("{}-peaks", end_ms - start_ms));
        Ok(Self {
            segment,
            track_id,
            source_kind,
            stream,
            offset_ms,
            start_ms,
            end_ms,
            path,
        })
    }
    pub(crate) fn maximum(&self) -> Result<f64, String> {
        let mut file = File::open(&self.path).map_err(|_| "waveform_pending".to_owned())?;
        let mut header = [0u8; 40];
        file.read_exact(&mut header)
            .map_err(|_| "waveform_cache_invalid".to_owned())?;
        let duration = i64::from_le_bytes(
            header[16..24]
                .try_into()
                .map_err(|_| "waveform_cache_invalid")?,
        );
        let maximum = f64::from_le_bytes(
            header[24..32]
                .try_into()
                .map_err(|_| "waveform_cache_invalid")?,
        );
        let expected =
            HEADER_BYTES + ((duration + BASE_BIN_MS - 1) / BASE_BIN_MS).max(0) as u64 * 8;
        if &header[..8] != MAGIC
            || i64::from_le_bytes(
                header[8..16]
                    .try_into()
                    .map_err(|_| "waveform_cache_invalid")?,
            ) != BASE_BIN_MS
            || duration != self.end_ms - self.start_ms
            || !maximum.is_finite()
            || maximum < 0.0
            || file.metadata().map_err(|_| "waveform_cache_invalid")?.len() != expected
        {
            return Err("waveform_cache_invalid".into());
        }
        Ok(maximum)
    }
    pub(crate) fn prepare(&self, cancel: &AtomicBool) -> Result<(), String> {
        if self.maximum().is_ok() {
            return super::pyramid::prepare(self, cancel);
        }
        let directory = self.path.parent().ok_or("waveform_cache_invalid")?;
        fs::create_dir_all(directory).map_err(|_| "waveform_cache_write_failed")?;
        let temporary = self
            .path
            .with_extension(format!("{}.partial", uuid::Uuid::now_v7()));
        let result = (|| {
            let mut file = BufWriter::new(
                File::create(&temporary).map_err(|_| "waveform_cache_write_failed")?,
            );
            file.write_all(&[0u8; 40])
                .map_err(|_| "waveform_cache_write_failed")?;
            let mut maximum = 0f64;
            let mut start = self.start_ms;
            while start < self.end_ms {
                if cancel.load(Ordering::SeqCst) {
                    return Err("ai_processing_cancelled".to_owned());
                }
                let duration = 30_000.min(self.end_ms - start);
                let source_start = start - self.offset_ms;
                let mut command = Command::new(crate::ffmpeg::binary_path());
                command.args([
                    "-hide_banner",
                    "-v",
                    "error",
                    "-threads",
                    "1",
                    "-i",
                    &self.segment.file_path,
                    "-ss",
                    &format!("{:.3}", source_start as f64 / 1000.0),
                    "-t",
                    &format!("{:.3}", duration as f64 / 1000.0),
                    "-map",
                    &format!("0:{}", self.stream),
                    "-af",
                    &crate::ffmpeg::audio_timeline_filter(48_000, source_start + duration),
                    "-ac",
                    "1",
                    "-ar",
                    "48000",
                    "-f",
                    "f32le",
                    "pipe:1",
                ]);
                let output = crate::subprocess::run(
                    &mut command,
                    cancel,
                    Duration::from_secs(60),
                    6_000_000,
                )
                .map_err(|error| {
                    if error.kind() == std::io::ErrorKind::Interrupted {
                        "ai_processing_cancelled"
                    } else {
                        "ai_waveform_generation_failed"
                    }
                })?;
                if !output.status.success() {
                    return Err("ai_waveform_generation_failed".into());
                }
                let samples = output
                    .stdout
                    .chunks_exact(4)
                    .map(|bytes| {
                        f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as f64
                    })
                    .collect::<Vec<_>>();
                let (peaks, rms) = waveform_peak_rms_time_bins(&samples, duration, BASE_BIN_MS);
                for (peak, rms) in peaks.into_iter().zip(rms) {
                    if !peak.is_finite() || !rms.is_finite() {
                        return Err("ai_waveform_generation_failed".into());
                    }
                    maximum = maximum.max(peak);
                    file.write_all(&(peak as f32).to_le_bytes())
                        .and_then(|_| file.write_all(&(rms as f32).to_le_bytes()))
                        .map_err(|_| "waveform_cache_write_failed")?;
                }
                start += duration;
            }
            file.seek(SeekFrom::Start(0))
                .map_err(|_| "waveform_cache_write_failed")?;
            file.write_all(MAGIC)
                .and_then(|_| file.write_all(&BASE_BIN_MS.to_le_bytes()))
                .and_then(|_| file.write_all(&(self.end_ms - self.start_ms).to_le_bytes()))
                .and_then(|_| file.write_all(&maximum.to_le_bytes()))
                .and_then(|_| file.write_all(&super::super::now_ms().to_le_bytes()))
                .and_then(|_| file.flush())
                .and_then(|_| file.get_ref().sync_all())
                .map_err(|_| "waveform_cache_write_failed")?;
            if cancel.load(Ordering::SeqCst) {
                return Err("ai_processing_cancelled".into());
            }
            fs::rename(&temporary, &self.path).map_err(|_| "waveform_cache_write_failed")?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(temporary);
        }
        result?;
        super::pyramid::prepare(self, cancel)
    }

    pub(crate) fn tile(
        &self,
        from: i64,
        to: i64,
        bin_ms: i64,
        scale: f64,
    ) -> Result<super::super::LibraryWaveformTile, String> {
        self.maximum()?;
        let start = from.max(self.start_ms);
        let end = to.min(self.end_ms);
        let (source_bin, first, data) =
            if let Some(coarse) = super::pyramid::read(self, start, end, bin_ms) {
                coarse
            } else {
                let first = (start - self.start_ms) / BASE_BIN_MS;
                let last = (end - self.start_ms + BASE_BIN_MS - 1) / BASE_BIN_MS;
                let mut file = File::open(&self.path).map_err(|_| "waveform_cache_invalid")?;
                file.seek(SeekFrom::Start(HEADER_BYTES + first as u64 * 8))
                    .map_err(|_| "waveform_cache_invalid")?;
                let mut data = vec![0; (last - first).max(0) as usize * 8];
                file.read_exact(&mut data)
                    .map_err(|_| "waveform_cache_invalid")?;
                (BASE_BIN_MS, first, data)
            };
        let count = ((end - start + bin_ms - 1) / bin_ms).max(0) as usize;
        let mut peaks = vec![0f64; count];
        let mut energy = vec![0f64; count];
        let mut weights = vec![0f64; count];
        for (index, bytes) in data.chunks_exact(8).enumerate() {
            let peak = f32::from_le_bytes(
                bytes[..4]
                    .try_into()
                    .map_err(|_| "waveform_cache_invalid")?,
            ) as f64;
            let rms = f32::from_le_bytes(
                bytes[4..]
                    .try_into()
                    .map_err(|_| "waveform_cache_invalid")?,
            ) as f64;
            if !peak.is_finite() || !rms.is_finite() || peak < 0.0 || rms < 0.0 {
                return Err(if source_bin == BASE_BIN_MS {
                    "waveform_cache_invalid"
                } else {
                    "waveform_pyramid_invalid"
                }
                .into());
            }
            let time = (self.start_ms + (first + index as i64) * source_bin).max(start);
            let bin = ((time - start) / bin_ms) as usize;
            if bin >= count {
                continue;
            }
            let weight = source_bin.min(end - time).max(0) as f64;
            peaks[bin] = peaks[bin].max(peak);
            energy[bin] += rms * rms * weight;
            weights[bin] += weight;
        }
        Ok(super::super::LibraryWaveformTile {
            bin_ms,
            duration_ms: end - start,
            generated_at_ms: super::super::now_ms(),
            peak_samples: peaks
                .into_iter()
                .map(|peak| (peak / scale.max(f64::EPSILON)).clamp(0.0, 1.0))
                .collect(),
            rms_samples: energy
                .into_iter()
                .zip(weights)
                .map(|(energy, weight)| {
                    ((energy / weight.max(1.0)).sqrt() / scale.max(f64::EPSILON)).clamp(0.0, 1.0)
                })
                .collect(),
            segment_id: self.segment.id.clone(),
            stream_index: if self.source_kind == "microphone" {
                self.segment
                    .microphone_audio_stream_index
                    .unwrap_or(self.stream)
            } else {
                self.stream
            },
            timeline_start_ms: start,
        })
    }
}
