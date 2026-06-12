use super::super::*;

#[derive(Clone, Debug)]
pub(in crate::ai) struct AudioChunkPlan {
    pub(in crate::ai) end_offset_ms: i64,
    pub(in crate::ai) segment_id: String,
    pub(in crate::ai) source_role: String,
    pub(in crate::ai) start_offset_ms: i64,
}

#[derive(Clone, Debug)]
pub(in crate::ai) struct ActivityChunkPlan {
    pub(in crate::ai) end_offset_ms: i64,
    pub(in crate::ai) segment_id: String,
    pub(in crate::ai) start_offset_ms: i64,
}

#[derive(Clone, Debug)]
pub(in crate::ai) struct VisualChunkPlan {
    pub(in crate::ai) end_offset_ms: i64,
    pub(in crate::ai) segment_id: String,
    pub(in crate::ai) start_offset_ms: i64,
}

pub(in crate::ai) fn audio_chunk_plans(segments: &[RecordingSegmentRow]) -> Vec<AudioChunkPlan> {
    let mut playback_offset_ms = 0;
    let mut plans = Vec::new();
    for (index, segment) in segments.iter().enumerate() {
        let duration_ms = segment.duration_ms.unwrap_or_else(|| {
            segment
                .ended_at_ms
                .map(|ended_at_ms| ended_at_ms.saturating_sub(segment.started_at_ms))
                .unwrap_or(0)
        });
        let next_started_at_ms = segments.get(index + 1).map(|next| next.started_at_ms);
        let pause_after_ms = segment
            .ended_at_ms
            .zip(next_started_at_ms)
            .map(|(ended_at_ms, next_started_at_ms)| next_started_at_ms.saturating_sub(ended_at_ms))
            .unwrap_or(0);
        if segment.state == "finalized" && Path::new(&segment.file_path).is_file() {
            if segment.desktop_audio_stream_index.is_some() {
                append_audio_slice_plans(
                    &mut plans,
                    segment,
                    "desktop-audio",
                    playback_offset_ms,
                    duration_ms.max(0),
                );
            }
            if segment.microphone_audio_stream_index.is_some() {
                append_audio_slice_plans(
                    &mut plans,
                    segment,
                    "microphone",
                    playback_offset_ms,
                    duration_ms.max(0),
                );
            }
            playback_offset_ms += duration_ms.max(0) + pause_after_ms.max(0);
        }
    }

    plans
}

pub(in crate::ai) fn activity_chunk_plans(
    segments: &[RecordingSegmentRow],
    app_settings: &settings::AppSettings,
) -> Vec<ActivityChunkPlan> {
    if !app_settings.ai.activity.frame_deltas.enabled
        && !app_settings.ai.activity.keyboard.enabled
        && !app_settings.ai.activity.mouse.enabled
    {
        return Vec::new();
    }

    let mut playback_offset_ms = 0;
    let mut plans = Vec::new();
    for (index, segment) in segments.iter().enumerate() {
        let duration_ms = segment.duration_ms.unwrap_or_else(|| {
            segment
                .ended_at_ms
                .map(|ended_at_ms| ended_at_ms.saturating_sub(segment.started_at_ms))
                .unwrap_or(0)
        });
        let next_started_at_ms = segments.get(index + 1).map(|next| next.started_at_ms);
        let pause_after_ms = segment
            .ended_at_ms
            .zip(next_started_at_ms)
            .map(|(ended_at_ms, next_started_at_ms)| next_started_at_ms.saturating_sub(ended_at_ms))
            .unwrap_or(0);
        if segment.state == "finalized" && Path::new(&segment.file_path).is_file() {
            plans.push(ActivityChunkPlan {
                end_offset_ms: playback_offset_ms + duration_ms.max(0),
                segment_id: segment.id.clone(),
                start_offset_ms: playback_offset_ms,
            });
            playback_offset_ms += duration_ms.max(0) + pause_after_ms.max(0);
        }
    }

    plans
}

pub(in crate::ai) fn visual_chunk_plans(
    segments: &[RecordingSegmentRow],
    app_settings: &settings::AppSettings,
    source_kind: &str,
) -> Vec<VisualChunkPlan> {
    let enabled = match source_kind {
        "ocr" => app_settings.ai.ocr.enabled,
        "vlm" => {
            app_settings.ai.ocr.enabled
                && app_settings.ai.vision.provider == "openai-compatible"
                && app_settings.ai.vision.base_url.is_some()
                && app_settings.ai.vision.api_key_ref.is_some()
                && app_settings.ai.vision.model.is_some()
        }
        _ => false,
    };
    if !enabled {
        return Vec::new();
    }

    let mut playback_offset_ms = 0;
    let mut plans = Vec::new();
    for (index, segment) in segments.iter().enumerate() {
        let duration_ms = segment.duration_ms.unwrap_or_else(|| {
            segment
                .ended_at_ms
                .map(|ended_at_ms| ended_at_ms.saturating_sub(segment.started_at_ms))
                .unwrap_or(0)
        });
        let next_started_at_ms = segments.get(index + 1).map(|next| next.started_at_ms);
        let pause_after_ms = segment
            .ended_at_ms
            .zip(next_started_at_ms)
            .map(|(ended_at_ms, next_started_at_ms)| next_started_at_ms.saturating_sub(ended_at_ms))
            .unwrap_or(0);
        if segment.state == "finalized"
            && segment.video_stream_index.is_some()
            && Path::new(&segment.file_path).is_file()
        {
            plans.push(VisualChunkPlan {
                end_offset_ms: playback_offset_ms + duration_ms.max(0),
                segment_id: segment.id.clone(),
                start_offset_ms: playback_offset_ms,
            });
        }
        if segment.state == "finalized" && Path::new(&segment.file_path).is_file() {
            playback_offset_ms += duration_ms.max(0) + pause_after_ms.max(0);
        }
    }

    plans
}

pub(in crate::ai) fn append_audio_slice_plans(
    plans: &mut Vec<AudioChunkPlan>,
    segment: &RecordingSegmentRow,
    source_role: &str,
    playback_offset_ms: i64,
    duration_ms: i64,
) {
    let mut slice_start_ms = 0;
    while slice_start_ms < duration_ms {
        let slice_end_ms = (slice_start_ms + MAX_TRANSCRIPTION_CHUNK_DURATION_MS).min(duration_ms);
        plans.push(AudioChunkPlan {
            end_offset_ms: playback_offset_ms + slice_end_ms,
            segment_id: segment.id.clone(),
            source_role: source_role.to_string(),
            start_offset_ms: playback_offset_ms + slice_start_ms,
        });
        slice_start_ms = slice_end_ms;
    }
}

pub(in crate::ai) fn playback_offset_for_segment(
    segments: &[RecordingSegmentRow],
    segment_id: &str,
) -> Option<i64> {
    let mut playback_offset_ms = 0;
    for (index, segment) in segments.iter().enumerate() {
        let duration_ms = segment.duration_ms.unwrap_or_else(|| {
            segment
                .ended_at_ms
                .map(|ended_at_ms| ended_at_ms.saturating_sub(segment.started_at_ms))
                .unwrap_or(0)
        });
        let next_started_at_ms = segments.get(index + 1).map(|next| next.started_at_ms);
        let pause_after_ms = segment
            .ended_at_ms
            .zip(next_started_at_ms)
            .map(|(ended_at_ms, next_started_at_ms)| next_started_at_ms.saturating_sub(ended_at_ms))
            .unwrap_or(0);

        if segment.id == segment_id {
            return Some(playback_offset_ms);
        }

        if segment.state == "finalized" && Path::new(&segment.file_path).is_file() {
            playback_offset_ms += duration_ms.max(0) + pause_after_ms.max(0);
        }
    }

    None
}

pub(in crate::ai) fn ensure_audio_track(
    connection: &rusqlite::Connection,
    recording_id: &str,
    kind: &str,
    label: &str,
    timestamp_ms: i64,
) -> rusqlite::Result<RecordingTrackRow> {
    if let Some(track) = storage::get_recording_track_by_kind(connection, recording_id, kind)? {
        return Ok(track);
    }
    storage::upsert_recording_track(
        connection,
        &UpsertRecordingTrackInput {
            codec: None,
            created_at_ms: timestamp_ms,
            id: Uuid::now_v7().to_string(),
            kind: kind.to_string(),
            label: label.to_string(),
            path: None,
            recording_id: recording_id.to_string(),
            source_id: None,
        },
    )
}
