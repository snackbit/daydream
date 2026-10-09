use super::*;
use std::{sync::atomic::AtomicBool, time::Duration};

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AdjacentFrameInput {
    source_track_id: Option<String>,
    recording_id: String,
    timestamp_ms: i64,
    direction: i8,
}

fn timestamps(stderr: &[u8], start_ms: i64) -> Vec<i64> {
    let mut result = String::from_utf8_lossy(stderr)
        .lines()
        .filter_map(|line| {
            if !line.contains("showinfo") {
                return None;
            }
            let value = line
                .split("pts_time:")
                .nth(1)?
                .split_whitespace()
                .next()?
                .parse::<f64>()
                .ok()?;
            (value.is_finite() && value >= 0.0)
                .then_some(start_ms + (value * 1000.0).round() as i64)
        })
        .collect::<Vec<_>>();
    result.sort_unstable();
    result.dedup();
    result
}

fn adjacent(storage: &AppStorage, input: AdjacentFrameInput) -> Result<Option<i64>, String> {
    if !matches!(input.direction, -1 | 1) || input.timestamp_ms < 0 {
        return Err("Invalid frame navigation request.".into());
    }
    let (rows, redactions, recorded_fps, mappings) = storage
        .with_connection(|connection| {
            Ok((
                storage::list_recording_segments(connection, &input.recording_id)?,
                storage::list_redactions(connection, &input.recording_id)?,
                storage::get_recording(connection, &input.recording_id)?.default_fps,
                storage::recording_video_streams(connection, &input.recording_id)?,
            ))
        })
        .map_err(|_| "Recording could not be read.")?;
    let deadline = std::time::Instant::now() + Duration::from_secs(15);
    let library_segments = library_segments(&rows);
    let mut segments = library_segments
        .iter()
        .filter(|segment| {
            segment.file_exists
                && segment.state == "finalized"
                && segment.video_stream_index.is_some()
        })
        .collect::<Vec<_>>();
    if input.direction < 0 {
        segments.reverse();
    }
    for segment in segments {
        let duration = segment.duration_ms.unwrap_or(0);
        let end = segment.playback_offset_ms + duration;
        if (input.direction > 0 && end <= input.timestamp_ms)
            || (input.direction < 0 && segment.playback_offset_ms >= input.timestamp_ms)
        {
            continue;
        }
        let Some(source) = mappings.get(&segment.id).and_then(|sources| {
            sources.iter().find(|source| {
                input
                    .source_track_id
                    .as_ref()
                    .is_none_or(|id| id == &source.track_id)
            })
        }) else {
            continue;
        };
        let relative = input.timestamp_ms - segment.playback_offset_ms;
        let neighborhood =
            ((1000.0 / source.fps.unwrap_or(recorded_fps).max(1.0 / 60.0)).ceil() as i64 * 2)
                .max(20_000);
        let mut cursor = relative.clamp(0, duration);
        loop {
            // Jump across protection cuts before decoding another bounded window.
            // The original timestamp remains the strict next/previous comparison.
            cursor = redaction_boundary(
                &redactions,
                &source.track_id,
                segment.playback_offset_ms,
                cursor,
                input.direction,
            )
            .clamp(0, duration);
            if (input.direction > 0 && cursor >= duration) || (input.direction < 0 && cursor <= 0) {
                break;
            }
            let start = cursor.saturating_sub(neighborhood).max(0);
            let window_end = if input.direction > 0 {
                (cursor + neighborhood + 100).min(duration)
            } else {
                (cursor + 100).min(duration)
            };
            let window = decode_window(
                &segment.file_path,
                source.stream_index,
                cursor,
                start,
                window_end - start,
                input.direction,
                deadline,
            )?;
            let visible = |point: &&i64| {
                **point >= 0
                    && **point < duration
                    && !crate::recording_privacy::range_redacted(
                        &redactions,
                        std::slice::from_ref(&source.track_id),
                        **point + segment.playback_offset_ms,
                        **point + segment.playback_offset_ms + 1,
                    )
            };
            let point = if input.direction > 0 {
                window
                    .points
                    .iter()
                    .filter(|point| **point > relative)
                    .find(visible)
            } else {
                window
                    .points
                    .iter()
                    .rev()
                    .filter(|point| **point < relative)
                    .find(visible)
            };
            if let Some(point) = point {
                return Ok(Some(*point + segment.playback_offset_ms));
            }
            let next = if input.direction > 0 {
                window.end.min(duration)
            } else {
                window.start.max(0)
            };
            if (input.direction > 0 && next <= cursor) || (input.direction < 0 && next >= cursor) {
                break;
            }
            cursor = next;
        }
    }
    Ok(None)
}

fn redaction_boundary(
    redactions: &[RedactionRow],
    track: &str,
    offset: i64,
    relative: i64,
    direction: i8,
) -> i64 {
    let tracks = [track.to_string()];
    let mut cursor = relative + offset;
    loop {
        // A backward step searches before the cursor; cut ends are exclusive.
        let probe = if direction > 0 {
            cursor
        } else {
            cursor.saturating_sub(1)
        };
        let next = redactions
            .iter()
            .filter(|cut| {
                crate::recording_privacy::range_redacted(
                    std::slice::from_ref(*cut),
                    &tracks,
                    probe,
                    probe.saturating_add(1),
                )
            })
            .map(|cut| {
                if direction > 0 {
                    cut.end_offset_ms
                } else {
                    cut.start_offset_ms
                }
            })
            .fold(cursor, |bound, next| {
                if direction > 0 {
                    bound.max(next)
                } else {
                    bound.min(next)
                }
            });
        if next == cursor {
            return cursor - offset;
        }
        cursor = next;
    }
}

fn decode_window(
    path: &str,
    stream: i64,
    target: i64,
    start: i64,
    length: i64,
    direction: i8,
    deadline: std::time::Instant,
) -> Result<FrameWindow, String> {
    let remaining = deadline.saturating_duration_since(std::time::Instant::now());
    if remaining.is_zero() {
        return Err("Frame navigation could not finish. Try again.".into());
    }
    let fingerprint = file_fingerprint(path).ok_or("Video file unavailable.")?;
    let cached = frame_cache()
        .lock()
        .map_err(|_| "Frame index unavailable.")?
        .iter()
        .find(|window| {
            window.stream_index == stream
                && window.path == path
                && window.fingerprint == fingerprint
                && target >= window.start
                && target <= window.end
                && window.points.iter().any(|point| {
                    if direction > 0 {
                        *point > target
                    } else {
                        *point < target
                    }
                })
        })
        .cloned();
    if let Some(window) = cached {
        return Ok(window);
    }
    let mut command = Command::new(ffmpeg::binary_path());
    command.args([
        "-hide_banner",
        "-nostdin",
        "-threads",
        "1",
        "-ss",
        &format!("{:.3}", start as f64 / 1000.0),
        "-i",
        path,
        "-map",
        &format!("0:{stream}"),
        "-t",
        &format!("{:.3}", length as f64 / 1000.0),
        "-vf",
        "showinfo=checksum=0",
        "-an",
        "-fps_mode",
        "passthrough",
        "-f",
        "null",
        "-",
    ]);
    let output =
        crate::subprocess::run(&mut command, &AtomicBool::new(false), remaining, 2_000_000)
            .map_err(|_| "Frame navigation could not finish. Try again.")?;
    if !output.status.success() {
        return Err("Video frames could not be read.".into());
    }
    let window = FrameWindow {
        stream_index: stream,
        path: path.into(),
        fingerprint,
        start,
        end: start + length,
        points: std::sync::Arc::new(timestamps(&output.stderr, start)),
    };
    let mut cache = frame_cache()
        .lock()
        .map_err(|_| "Frame index unavailable.")?;
    if cache.len() >= 16 {
        cache.pop_front();
    }
    cache.push_back(window.clone());
    Ok(window)
}

#[derive(Clone)]
struct FrameWindow {
    stream_index: i64,
    path: String,
    fingerprint: (u64, i64),
    start: i64,
    end: i64,
    points: std::sync::Arc<Vec<i64>>,
}
fn frame_cache() -> &'static std::sync::Mutex<std::collections::VecDeque<FrameWindow>> {
    static CACHE: std::sync::OnceLock<std::sync::Mutex<std::collections::VecDeque<FrameWindow>>> =
        std::sync::OnceLock::new();
    CACHE.get_or_init(|| std::sync::Mutex::new(std::collections::VecDeque::new()))
}

#[tauri::command(rename = "adjacentLibraryFrame")]
pub(crate) async fn adjacent_library_frame(
    app: tauri::AppHandle,
    input: AdjacentFrameInput,
) -> Result<Option<i64>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let _activity = crate::maintenance::runtime::admit(&app, "frame-read")?;
        let operation = crate::background_tasks::operations::Operation::start(
            &app,
            "frame-index",
            &input.recording_id,
            "Video frame navigation",
        );
        let result = adjacent(&app.state::<AppStorage>(), input);
        operation.finish(result.as_ref().err().map(|_| "ai_frame_index_failed"));
        result
    })
    .await
    .map_err(|_| "Frame navigation worker failed.".to_string())?
}

#[cfg(test)]
mod tests;
