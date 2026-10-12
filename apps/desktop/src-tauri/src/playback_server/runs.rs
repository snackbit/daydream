//! Conservative run grouping. Media fragments and their provenance remain intact.
use crate::storage::{RecordingSegmentRow, SegmentVideoStream};
use std::{collections::BTreeMap, path::Path};

pub(super) fn groups<'a>(
    segments: &'a [RecordingSegmentRow],
    sources: &BTreeMap<String, Vec<SegmentVideoStream>>,
) -> Vec<Vec<&'a RecordingSegmentRow>> {
    let mut groups: Vec<Vec<&RecordingSegmentRow>> = Vec::new();
    let mut previous: Option<&RecordingSegmentRow> = None;
    for segment in segments {
        if segment.state != "finalized"
            || !Path::new(&segment.file_path).is_file()
            || segment.duration_ms.is_none_or(|duration| duration <= 0)
        {
            previous = None;
            continue;
        }
        let compatible = previous.is_some_and(|previous| {
            let continuous = match (&previous.capture_run_id, &segment.capture_run_id) {
                (Some(a), Some(b)) => a == b,
                (None, None) => {
                    segment
                        .started_at_ms
                        .checked_sub(previous.ended_at_ms.unwrap_or(i64::MIN))
                        .is_some_and(|gap| (0..=50).contains(&gap))
                        && previous
                            .timeline_base_ms
                            .zip(segment.timeline_base_ms)
                            .is_some_and(|(a, b)| {
                                b.checked_sub(a)
                                    .and_then(|elapsed| {
                                        elapsed.checked_sub(previous.duration_ms.unwrap_or(0))
                                    })
                                    .is_some_and(|drift| drift.unsigned_abs() <= 50)
                            })
                }
                _ => false,
            };
            continuous
                && previous.segment_index.checked_add(1) == Some(segment.segment_index)
                && previous.video_stream_index == segment.video_stream_index
                && previous.desktop_audio_stream_index == segment.desktop_audio_stream_index
                && previous.microphone_audio_stream_index == segment.microphone_audio_stream_index
                && sources.get(&previous.id).is_some_and(|streams| {
                    !streams.is_empty()
                        && streams.iter().all(|stream| {
                            stream.width.is_some()
                                && stream.height.is_some()
                                && stream.fps.is_some()
                        })
                        && Some(streams) == sources.get(&segment.id)
                })
        });
        if compatible {
            groups.last_mut().expect("previous group").push(segment);
        } else {
            groups.push(vec![segment]);
        }
        previous = Some(segment);
    }
    groups
}

pub(crate) fn concat_manifest(fragments: &[(String, i64)]) -> Result<String, String> {
    let mut manifest = String::from("ffconcat version 1.0\n");
    for (path, duration) in fragments {
        if *duration <= 0 {
            return Err("Playback timing unavailable.".into());
        }
        let path = std::fs::canonicalize(path).map_err(|_| "Playback media unavailable.")?;
        if !path.is_file() {
            return Err("Playback media unavailable.".into());
        }
        let path = path.to_str().ok_or("Playback path unavailable.")?;
        if path.contains(['\n', '\r', '\0']) {
            return Err("Playback path unavailable.".into());
        }
        manifest.push_str(&format!(
            "file '{}'\nduration {:.3}\n",
            path.replace('\'', "'\\''"),
            *duration as f64 / 1000.
        ));
    }
    Ok(manifest)
}
