//! Readiness labels reuse the same source plans as worker admission.
use super::super::*;

pub(crate) fn source_applicability(
    connection: &rusqlite::Connection,
    recording_id: &str,
    kind: &str,
    settings: &settings::AppSettings,
) -> rusqlite::Result<String> {
    let enabled = match kind {
        "audio" => settings.ai.transcription.provider != "disabled",
        "waveform" => true,
        "ocr" => settings.ai.ocr.enabled,
        "vlm" => settings.ai.vision.observations_enabled,
        "activity" => {
            settings.ai.activity.frame_deltas.enabled
                || settings.ai.activity.keyboard.enabled
                || settings.ai.activity.mouse.enabled
        }
        _ => return Err(rusqlite::Error::InvalidQuery),
    };
    if !enabled {
        return Ok("disabled".into());
    }
    if !crate::recording_privacy::protection_ready(connection, recording_id)? {
        return Ok("protected".into());
    }
    let recording = storage::get_recording(connection, recording_id)?;
    let segments = storage::list_recording_segments(connection, recording_id)?;
    let has_source = segments.iter().any(|segment| match kind {
        "audio" | "waveform" => {
            segment.desktop_audio_stream_index.is_some()
                || segment.microphone_audio_stream_index.is_some()
        }
        "ocr" | "vlm" => segment.video_stream_index.is_some(),
        _ => segment.state == "finalized",
    });
    if !has_source {
        return Ok(
            if matches!(recording.status.as_str(), "recording" | "paused") {
                "waiting"
            } else {
                "no-source"
            }
            .into(),
        );
    }
    let redactions = storage::list_redactions(connection, recording_id)?;
    let eligible = match kind {
        "audio" | "waveform" => {
            !super::planning::audio_chunk_plans(&segments, &redactions).is_empty()
        }
        "activity" => {
            !super::planning::activity_chunk_plans(&segments, settings, &redactions).is_empty()
        }
        _ => !super::source_planning::visual_source_chunk_plans(
            connection,
            &segments,
            settings,
            &redactions,
            kind,
        )?
        .is_empty(),
    };
    Ok(if eligible {
        "available"
    } else {
        "no-eligible-input"
    }
    .into())
}
