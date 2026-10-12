use super::{super::super::*, types::*};
use rusqlite::Connection;

pub(super) fn plan(
    connection: &Connection,
    settings: &settings::AppSettings,
    input: &PreviewRecordingReprocessingInput,
) -> rusqlite::Result<Plan> {
    if input.targets.is_empty() || input.targets.len() > 128 {
        return Err(rusqlite::Error::InvalidQuery);
    }
    let recording = storage::get_recording(connection, &input.recording_id)?;
    let blocked = if !matches!(
        recording.status.as_str(),
        "completed" | "failed" | "recovered-partial"
    ) {
        Some("Finish recording before reprocessing.")
    } else if !crate::recording_privacy::protection_ready(connection, &input.recording_id)? {
        Some("Recording protection must finish before reprocessing.")
    } else if storage::list_processing_jobs(connection, Some(&input.recording_id))?
        .iter()
        .any(|job| matches!(job.status.as_str(), "queued" | "running" | "paused"))
    {
        Some("Stop active processing before reprocessing.")
    } else {
        None
    };
    let segments = storage::list_recording_segments(connection, &input.recording_id)?;
    let mut known_sources = BTreeSet::new();
    for segment in &segments {
        for role in ["microphone", "desktop-audio"] {
            if let Some(id) = crate::recording_privacy::source_track_id(segment, role) {
                known_sources.insert(id);
            }
        }
        for source in storage::segment_video_streams(connection, segment)? {
            known_sources.insert(source.track_id);
        }
    }
    let redactions = storage::list_redactions(connection, &input.recording_id)?;
    let mut available = Vec::new();
    for item in audio_chunk_plans(&segments, &redactions) {
        let segment = segments
            .iter()
            .find(|s| s.id == item.segment_id)
            .ok_or(rusqlite::Error::InvalidQuery)?;
        let source = crate::recording_privacy::source_track_id(segment, &item.source_role)
            .ok_or(rusqlite::Error::InvalidQuery)?;
        available.push(PlannedSection {
            segment_id: item.segment_id,
            native_track_id: None,
            role: Some(item.source_role.clone()),
            source_track_id: source,
            label: if item.source_role == "microphone" {
                "Microphone"
            } else {
                "Desktop audio"
            }
            .into(),
            operation: ReprocessingOperation::Transcription,
            start: item.start_offset_ms,
            end: item.end_offset_ms,
        });
    }
    // Enumerate disabled operations too, so the preview can explain their settings.
    let mut all_settings = settings.clone();
    all_settings.ai.ocr.enabled = true;
    all_settings.ai.vision.observations_enabled = true;
    for operation in [
        ReprocessingOperation::Ocr,
        ReprocessingOperation::ScreenObservations,
    ] {
        for item in visual_source_chunk_plans(
            connection,
            &segments,
            &all_settings,
            &redactions,
            operation.kind(),
        )? {
            let segment = segments
                .iter()
                .find(|s| s.id == item.segment_id)
                .ok_or(rusqlite::Error::InvalidQuery)?;
            let streams = storage::segment_video_streams(connection, segment)?;
            let stream = streams
                .iter()
                .find(|s| {
                    item.track_id
                        .as_ref()
                        .map_or(s.source_id.is_none(), |id| id == &s.track_id)
                })
                .ok_or(rusqlite::Error::InvalidQuery)?;
            available.push(PlannedSection {
                segment_id: item.segment_id,
                native_track_id: item.track_id,
                role: None,
                source_track_id: stream.track_id.clone(),
                label: stream.label.clone(),
                operation: operation.clone(),
                start: item.start_offset_ms,
                end: item.end_offset_ms,
            });
        }
    }
    let supported = available
        .iter()
        .filter(|s| {
            input
                .targets
                .iter()
                .any(|t| t.track_id == s.source_track_id)
        })
        .map(|s| s.operation.clone())
        .collect::<BTreeSet<_>>();
    let operations = supported
        .iter()
        .map(|operation| {
            let (enabled, remote, provider) = match operation {
                ReprocessingOperation::Transcription => (
                    settings.ai.transcription.provider != "disabled",
                    !matches!(
                        settings.ai.transcription.provider.as_str(),
                        "disabled" | "local-whisper-cpp"
                    ),
                    settings.ai.transcription.provider.clone(),
                ),
                ReprocessingOperation::Ocr => (
                    settings.ai.ocr.enabled,
                    settings.ai.ocr.engine == "vision",
                    settings.ai.ocr.engine.clone(),
                ),
                ReprocessingOperation::ScreenObservations => (
                    settings.ai.vision.observations_enabled,
                    true,
                    settings.ai.vision.provider.clone(),
                ),
            };
            ReprocessingOperationInfo {
                operation: operation.clone(),
                enabled,
                remote,
                provider_label: provider,
                blocked_reason: if enabled {
                    None
                } else {
                    Some("Enable this operation in model settings first.".into())
                },
            }
        })
        .collect::<Vec<_>>();
    let selected = input.operations.clone().unwrap_or_else(|| {
        operations
            .iter()
            .filter(|o| o.enabled)
            .map(|o| o.operation.clone())
            .collect()
    });
    if selected.iter().any(|o| !supported.contains(o)) {
        return Err(rusqlite::Error::InvalidQuery);
    }
    let transcript = storage::list_transcript_segments(connection, &input.recording_id)?;
    let utterances =
        super::utterances::utterance_ranges(connection, &input.recording_id, &transcript)?;
    let mut requested = Vec::new();
    for target in &input.targets {
        if !known_sources.contains(&target.track_id) {
            return Err(rusqlite::Error::InvalidQuery);
        }
        if target.ranges.as_ref().is_some_and(|r| {
            r.is_empty()
                || r.len() > 256
                || r.iter()
                    .any(|r| r.start_offset_ms < 0 || r.end_offset_ms <= r.start_offset_ms)
        }) {
            return Err(rusqlite::Error::InvalidQuery);
        }
        for section in available
            .iter()
            .filter(|s| s.source_track_id == target.track_id && selected.contains(&s.operation))
        {
            let ranges = target.ranges.clone().unwrap_or_else(|| {
                vec![ReprocessingRange {
                    start_offset_ms: section.start,
                    end_offset_ms: section.end,
                }]
            });
            for range in ranges {
                let mut start = section.start.max(range.start_offset_ms);
                let mut end = section.end.min(range.end_offset_ms);
                if end <= start {
                    continue;
                }
                if let Some(role) = &section.role {
                    // Expand through connected utterances before applying current privacy cuts.
                    if let Some(ranges) = utterances.get(role) {
                        (start, end) = super::utterances::expand(start, end, ranges);
                    }
                }
                requested.push((
                    section.source_track_id.clone(),
                    section.operation.clone(),
                    start,
                    end,
                ));
            }
        }
    }
    requested.sort();
    let mut merged: Vec<(String, ReprocessingOperation, i64, i64)> = Vec::new();
    for range in requested {
        if let Some(last) = merged.last_mut() {
            if last.0 == range.0 && last.1 == range.1 && range.2 < last.3 {
                last.3 = last.3.max(range.3);
                continue;
            }
        }
        merged.push(range)
    }
    let mut sections = Vec::new();
    let mut scopes = Vec::new();
    for (source, operation, start, end) in merged {
        for (start, end) in visible_ranges(start, end, &redactions, Some(&source)) {
            let children = available
                .iter()
                .filter(|s| {
                    s.source_track_id == source
                        && s.operation == operation
                        && s.start < end
                        && s.end > start
                })
                .collect::<Vec<_>>();
            if children.is_empty() {
                continue;
            }
            let actual_start = children
                .iter()
                .map(|s| s.start.max(start))
                .min()
                .unwrap_or(start);
            let actual_end = children.iter().map(|s| s.end.min(end)).max().unwrap_or(end);
            let id = format!(
                "{}:{}:{}:{}",
                source,
                operation.kind(),
                actual_start,
                actual_end
            );
            scopes.push(RecordingReprocessingScope {
                id,
                track_id: source.clone(),
                source_label: children[0].label.clone(),
                operation: operation.clone(),
                start_offset_ms: actual_start,
                end_offset_ms: actual_end,
            });
            for child in children {
                sections.push(PlannedSection {
                    segment_id: child.segment_id.clone(),
                    native_track_id: child.native_track_id.clone(),
                    role: child.role.clone(),
                    source_track_id: source.clone(),
                    label: child.label.clone(),
                    operation: operation.clone(),
                    start: child.start.max(start),
                    end: child.end.min(end),
                });
            }
        }
    }
    let blocked = blocked.map(str::to_string).or_else(|| {
        if scopes.is_empty() {
            Some("Choose an enabled operation and an eligible media range.".into())
        } else if selected
            .iter()
            .any(|s| operations.iter().any(|o| &o.operation == s && !o.enabled))
        {
            Some("Enable the selected operations in model settings first.".into())
        } else {
            None
        }
    });
    let cleanup_revision = if sections.iter().any(|section| {
        section.operation == ReprocessingOperation::Transcription
            && section.role.as_deref() == Some("microphone")
    }) {
        crate::library::audio_cleanup::active_manifest(connection, &input.recording_id)?
            .map(|manifest| manifest.revision)
    } else {
        None
    };
    let fingerprint = format!(
        "{:x}",
        Sha1::digest(
            serde_json::to_vec(&(
                settings,
                &cleanup_revision,
                &scopes,
                &sections,
                &redactions
                    .iter()
                    .map(|r| (&r.id, r.start_offset_ms, r.end_offset_ms, &r.track_ids_json))
                    .collect::<Vec<_>>(),
                &transcript
                    .iter()
                    .map(|t| (&t.id, t.start_offset_ms, t.end_offset_ms))
                    .collect::<Vec<_>>()
            ))
            .map_err(|_| rusqlite::Error::InvalidQuery)?
        )
    );
    Ok(Plan {
        preview: RecordingReprocessingPreview {
            recording_id: input.recording_id.clone(),
            plan_fingerprint: fingerprint,
            scopes,
            operations,
            blocked_reason: blocked,
        },
        sections,
    })
}
