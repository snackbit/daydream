//! Coarse peaks reuse the persisted envelope; preparation never decodes audio.
use super::prepared::{PreparedSource, BASE_BIN_MS};
use std::{
    fs::{self, File},
    io::{Read, Seek, SeekFrom, Write},
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
};
const LEVELS: [i64; 5] = [25, 100, 500, 2500, 10_000];
const MAGIC: &[u8; 8] = b"DWFMP001";
const HEADER: u64 = 24 + 5 * 24;

fn path(source: &PreparedSource) -> PathBuf {
    source.path.with_extension("pyramid-v1")
}
pub(super) fn ready(source: &PreparedSource) -> bool {
    let valid = || -> Option<bool> {
        let mut file = File::open(path(source)).ok()?;
        let mut header = [0u8; HEADER as usize];
        file.read_exact(&mut header).ok()?;
        if &header[..8] != MAGIC {
            return Some(false);
        }
        let duration = i64::from_le_bytes(header[8..16].try_into().ok()?);
        let maximum = f64::from_le_bytes(header[16..24].try_into().ok()?);
        if duration != source.end_ms - source.start_ms || !maximum.is_finite() || maximum < 0. {
            return Some(false);
        }
        let mut offset = HEADER;
        for (index, bin) in LEVELS.iter().enumerate() {
            let i = 24 + index * 24;
            let count = (duration + bin - 1) / bin;
            if i64::from_le_bytes(header[i..i + 8].try_into().ok()?) != *bin
                || u64::from_le_bytes(header[i + 8..i + 16].try_into().ok()?) != offset
                || i64::from_le_bytes(header[i + 16..i + 24].try_into().ok()?) != count
            {
                return Some(false);
            }
            offset += count as u64 * 8;
        }
        Some(file.metadata().ok()?.len() == offset)
    };
    valid() == Some(true)
}

pub(super) fn prepare(source: &PreparedSource, cancel: &AtomicBool) -> Result<(), String> {
    let destination = path(source);
    if ready(source) {
        return Ok(());
    }
    let maximum = source.maximum()?;
    let duration = source.end_ms - source.start_ms;
    let mut input = File::open(&source.path).map_err(|_| "waveform_cache_invalid")?;
    input
        .seek(SeekFrom::Start(40))
        .map_err(|_| "waveform_cache_invalid")?;
    let mut bytes = Vec::new();
    input
        .read_to_end(&mut bytes)
        .map_err(|_| "waveform_cache_invalid")?;
    let temporary = destination.with_extension(format!("{}.partial", uuid::Uuid::now_v7()));
    let result = (|| {
        let mut file = File::create(&temporary).map_err(|_| "waveform_cache_write_failed")?;
        file.write_all(MAGIC)
            .and_then(|_| file.write_all(&duration.to_le_bytes()))
            .and_then(|_| file.write_all(&maximum.to_le_bytes()))
            .map_err(|_| "waveform_cache_write_failed")?;
        let mut offset = HEADER;
        for bin in LEVELS {
            let count = (duration + bin - 1) / bin;
            file.write_all(&bin.to_le_bytes())
                .and_then(|_| file.write_all(&offset.to_le_bytes()))
                .and_then(|_| file.write_all(&count.to_le_bytes()))
                .map_err(|_| "waveform_cache_write_failed")?;
            offset += count as u64 * 8;
        }
        for bin in LEVELS {
            if cancel.load(Ordering::Acquire) {
                return Err("ai_processing_cancelled");
            }
            for (index, samples) in bytes
                .chunks(bin as usize / BASE_BIN_MS as usize * 8)
                .enumerate()
            {
                let mut peak = 0f32;
                let mut energy = 0f64;
                let mut weight = 0i64;
                for (i, pair) in samples.chunks_exact(8).enumerate() {
                    let p = f32::from_le_bytes(
                        pair[..4].try_into().map_err(|_| "waveform_cache_invalid")?,
                    );
                    let rms = f32::from_le_bytes(
                        pair[4..].try_into().map_err(|_| "waveform_cache_invalid")?,
                    );
                    if !p.is_finite() || !rms.is_finite() || p < 0. || rms < 0. {
                        return Err("waveform_cache_invalid");
                    }
                    let w = BASE_BIN_MS
                        .min(duration - index as i64 * bin - i as i64 * BASE_BIN_MS)
                        .max(0);
                    peak = peak.max(p);
                    energy += (rms as f64).powi(2) * w as f64;
                    weight += w;
                }
                let rms = (energy / weight.max(1) as f64).sqrt() as f32;
                file.write_all(&peak.to_le_bytes())
                    .and_then(|_| file.write_all(&rms.to_le_bytes()))
                    .map_err(|_| "waveform_cache_write_failed")?;
            }
        }
        file.sync_all().map_err(|_| "waveform_cache_write_failed")?;
        if cancel.load(Ordering::Acquire) {
            return Err("ai_processing_cancelled");
        }
        fs::rename(&temporary, &destination).map_err(|_| "waveform_cache_write_failed")
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result.map_err(str::to_owned)
}

pub(super) fn read(
    source: &PreparedSource,
    start: i64,
    end: i64,
    requested: i64,
) -> Option<(i64, i64, Vec<u8>)> {
    // Every aggregate must fit within one output bin, including viewport edges.
    // Otherwise use the base envelope rather than moving peaks across boundaries.
    let selected = LEVELS.iter().rposition(|bin| {
        requested % bin == 0
            && (start - source.start_ms) % bin == 0
            && ((end - source.start_ms) % bin == 0 || end == source.end_ms)
    })?;
    let mut file = File::open(path(source)).ok()?;
    let mut header = [0u8; HEADER as usize];
    file.read_exact(&mut header).ok()?;
    if &header[..8] != MAGIC
        || i64::from_le_bytes(header[8..16].try_into().ok()?) != source.end_ms - source.start_ms
    {
        return None;
    }
    let i = 24 + selected * 24;
    let bin = i64::from_le_bytes(header[i..i + 8].try_into().ok()?);
    if bin != LEVELS[selected] {
        return None;
    }
    let offset = u64::from_le_bytes(header[i + 8..i + 16].try_into().ok()?);
    let count = i64::from_le_bytes(header[i + 16..i + 24].try_into().ok()?);
    let expected = (source.end_ms - source.start_ms + bin - 1) / bin;
    if count != expected
        || offset < HEADER
        || offset.checked_add(count as u64 * 8)? > file.metadata().ok()?.len()
    {
        return None;
    }
    let first = (start - source.start_ms) / bin;
    let last = (end - source.start_ms + bin - 1) / bin;
    let mut data = vec![0; (last - first) as usize * 8];
    file.seek(SeekFrom::Start(offset + first as u64 * 8)).ok()?;
    file.read_exact(&mut data).ok()?;
    Some((bin, first, data))
}
