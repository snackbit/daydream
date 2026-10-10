use super::super::*;

pub(in crate::ai) fn transcript_for_recording(
    storage: &AppStorage,
    recording_id: &str,
) -> Result<RecordingTranscript, AiError> {
    let segments = visible_transcript_segments_for_recording(storage, recording_id)?;
    let mut words_by_segment = BTreeMap::new();
    for segment in &segments {
        let words = storage
            .with_connection(|connection| storage::list_transcript_words(connection, &segment.id))
            .map_err(storage_error)?;
        words_by_segment.insert(segment.id.clone(), words);
    }
    Ok(transcript_from_rows(
        recording_id,
        segments,
        words_by_segment,
    ))
}

pub(crate) fn visible_transcript_segments_for_recording(
    storage: &AppStorage,
    recording_id: &str,
) -> Result<Vec<TranscriptSegmentRow>, AiError> {
    let (segments, redactions, source_segments) = storage
        .with_connection(|connection| {
            Ok((
                storage::list_transcript_segments(connection, recording_id)?,
                storage::list_redactions(connection, recording_id)?,
                storage::list_recording_segments(connection, recording_id)?,
            ))
        })
        .map_err(storage_error)?;

    Ok(filter_visible_transcript_segments(
        segments,
        &redactions,
        &source_segments,
    ))
}

fn filter_visible_transcript_segments(
    segments: Vec<TranscriptSegmentRow>,
    redactions: &[RedactionRow],
    source_segments: &[RecordingSegmentRow],
) -> Vec<TranscriptSegmentRow> {
    if redactions.is_empty() {
        return segments;
    }

    segments
        .into_iter()
        .filter(|segment| {
            let mut targets = vec![segment.track_id.clone()];
            if let Some(source) = source_segments
                .iter()
                .find(|source| source.id == segment.segment_id)
            {
                if let Some(id) =
                    crate::recording_privacy::source_track_id(source, &segment.source_role)
                {
                    targets.push(id);
                }
            }
            !crate::recording_privacy::range_redacted(
                redactions,
                &targets,
                segment.start_offset_ms,
                segment.end_offset_ms,
            )
        })
        .collect()
}

pub(super) fn transcription_tracks_for_recording(
    _recording_id: &str,
    recording_segments: &[RecordingSegmentRow],
    recording_tracks: &[RecordingTrackRow],
    chunks: &[ProcessingChunk],
    transcript: &RecordingTranscript,
) -> Vec<RecordingTranscriptionTrackState> {
    let track_roles_by_id = recording_tracks
        .iter()
        .map(|track| (track.id.clone(), track.kind.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut source_roles = audio_source_roles_from_segments(recording_segments);
    for role in chunks.iter().filter_map(|chunk| {
        chunk
            .track_id
            .as_ref()
            .and_then(|track_id| track_roles_by_id.get(track_id))
            .cloned()
    }) {
        source_roles.insert(role);
    }
    for role in transcript
        .tracks
        .iter()
        .map(|track| track.source_role.clone())
    {
        source_roles.insert(role);
    }

    source_roles
        .into_iter()
        .filter_map(|source_role| {
            if !matches!(source_role.as_str(), "desktop-audio" | "microphone") {
                return None;
            }
            let track_id = recording_tracks
                .iter()
                .find(|track| track.kind == source_role)
                .map(|track| track.id.clone());
            let library_track_id = library_audio_track_id(recording_segments, &source_role);
            let transcript_segments = transcript
                .tracks
                .iter()
                .filter(|track| track.source_role == source_role)
                .flat_map(|track| track.segments.clone())
                .collect::<Vec<_>>();
            let mut assigned_segments = BTreeSet::new();
            let track_chunks = chunks
                .iter()
                .filter(|chunk| {
                    chunk.source_kind == "audio"
                        && chunk.track_id.as_ref().is_some_and(|chunk_track_id| {
                            track_roles_by_id
                                .get(chunk_track_id)
                                .is_some_and(|role| role == &source_role)
                        })
                })
                .map(|chunk| {
                    // Partially covered historical chunks retain their identity
                    // and status. Each canonical utterance belongs to the first
                    // matching (newest) chunk so overlapping history adds no
                    // duplicate transcript buttons to the timeline.
                    let owned = transcript_segments
                        .iter()
                        .filter(|segment| {
                            segment.start_offset_ms < chunk.end_offset_ms.unwrap_or(0)
                                && segment.end_offset_ms > chunk.start_offset_ms.unwrap_or(0)
                                && assigned_segments.insert(segment.id.clone())
                        })
                        .cloned()
                        .collect::<Vec<_>>();
                    transcription_chunk_state(chunk, &owned)
                })
                .collect::<Vec<_>>();
            let (status, status_label) =
                transcription_track_status(&track_chunks, library_track_id.is_none());

            Some(RecordingTranscriptionTrackState {
                chunks: track_chunks,
                description: transcription_track_description(
                    &source_role,
                    library_track_id.as_deref(),
                ),
                label: transcription_track_label(&source_role).to_string(),
                library_track_id,
                source_role: source_role.clone(),
                status,
                status_label,
                text_segment_count: transcript_segments.len(),
                track_id,
                track_key: source_role,
            })
        })
        .collect()
}

fn audio_source_roles_from_segments(segments: &[RecordingSegmentRow]) -> BTreeSet<String> {
    let mut roles = BTreeSet::new();
    for segment in segments {
        if segment.desktop_audio_stream_index.is_some() {
            roles.insert("desktop-audio".to_string());
        }
        if segment.microphone_audio_stream_index.is_some() {
            roles.insert("microphone".to_string());
        }
    }
    roles
}

fn library_audio_track_id(segments: &[RecordingSegmentRow], source_role: &str) -> Option<String> {
    let stream_index = segments.iter().find_map(|segment| match source_role {
        "desktop-audio" => segment.desktop_audio_stream_index,
        "microphone" => segment.microphone_audio_stream_index,
        _ => None,
    })?;
    Some(format!("{source_role}-{stream_index}"))
}

pub(in crate::ai) fn transcription_chunk_state(
    chunk: &ProcessingChunk,
    transcript_segments: &[RecordingTranscriptSegment],
) -> RecordingTranscriptionChunkState {
    let chunk_start_ms = chunk.start_offset_ms.unwrap_or(0);
    let chunk_end_ms = chunk.end_offset_ms.unwrap_or(chunk_start_ms);
    let chunk_segments = transcript_segments
        .iter()
        .filter(|segment| {
            segment.start_offset_ms < chunk_end_ms && segment.end_offset_ms > chunk_start_ms
        })
        .cloned()
        .collect::<Vec<_>>();
    let text = chunk_segments
        .iter()
        .map(|segment| segment.text.trim())
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    let result = match chunk.status.as_str() {
        "queued" => "queued",
        "running" => "running",
        "failed" => "failed",
        "cancelled" => "cancelled",
        "succeeded" if chunk.result_code.as_deref() == Some("skipped-silence") => "skipped-silence",
        "succeeded" if text.is_empty() => "no-speech",
        "succeeded" => "text",
        _ => "unknown",
    }
    .to_string();

    RecordingTranscriptionChunkState {
        attempt_count: chunk.attempt_count,
        end_offset_ms: chunk.end_offset_ms,
        error_code: chunk.error_code.clone(),
        id: chunk.id.clone(),
        progress: chunk.progress,
        result,
        result_code: chunk.result_code.clone(),
        segment_id: chunk.segment_id.clone(),
        start_offset_ms: chunk.start_offset_ms,
        status: chunk.status.clone(),
        text: (!text.is_empty()).then_some(text),
        transcript_segments: chunk_segments,
    }
}

fn transcription_track_status(
    chunks: &[RecordingTranscriptionChunkState],
    missing_audio_source: bool,
) -> (String, String) {
    if missing_audio_source {
        return ("no-audio".to_string(), "No audio source".to_string());
    }
    if chunks.is_empty() {
        return ("ready".to_string(), "Ready to process".to_string());
    }
    if chunks
        .iter()
        .any(|chunk| matches!(chunk.result.as_str(), "queued" | "running"))
    {
        return (
            "in-progress".to_string(),
            "Transcription in progress".to_string(),
        );
    }
    if chunks.iter().any(|chunk| chunk.result == "failed") {
        return ("failed".to_string(), "Transcription failed".to_string());
    }
    if chunks.iter().any(|chunk| chunk.result == "cancelled") {
        return (
            "cancelled".to_string(),
            "Transcription cancelled".to_string(),
        );
    }
    if chunks.iter().any(|chunk| chunk.result == "text") {
        return ("complete".to_string(), "Transcription complete".to_string());
    }
    ("no-speech".to_string(), "No speech detected".to_string())
}

fn transcription_track_label(source_role: &str) -> &str {
    match source_role {
        "desktop-audio" => "Desktop audio transcription",
        "microphone" => "Microphone transcription",
        _ => "Audio transcription",
    }
}

fn transcription_track_description(source_role: &str, library_track_id: Option<&str>) -> String {
    match library_track_id {
        Some(track_id) => format!("{source_role} timeline track {track_id}"),
        None => format!("{source_role} has no library audio track"),
    }
}

fn transcript_from_rows(
    recording_id: &str,
    segments: Vec<TranscriptSegmentRow>,
    words_by_segment: BTreeMap<String, Vec<TranscriptWordRow>>,
) -> RecordingTranscript {
    let mut tracks: BTreeMap<(String, String), Vec<RecordingTranscriptSegment>> = BTreeMap::new();
    for segment in segments {
        tracks
            .entry((segment.track_id.clone(), segment.source_role.clone()))
            .or_default()
            .push(RecordingTranscriptSegment {
                confidence: segment.confidence,
                end_offset_ms: segment.end_offset_ms,
                id: segment.id.clone(),
                model: segment.model,
                provider: segment.provider,
                segment_id: segment.segment_id,
                start_offset_ms: segment.start_offset_ms,
                text: segment.text,
                words: words_by_segment
                    .get(&segment.id)
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .map(transcript_word_from_row)
                    .collect(),
            });
    }

    RecordingTranscript {
        recording_id: recording_id.to_string(),
        tracks: tracks
            .into_iter()
            .map(
                |((track_id, source_role), segments)| RecordingTranscriptTrack {
                    source_role,
                    track_id,
                    segments,
                },
            )
            .collect(),
    }
}

fn transcript_word_from_row(row: TranscriptWordRow) -> RecordingTranscriptWord {
    RecordingTranscriptWord {
        end_offset_ms: row.end_offset_ms,
        id: row.id,
        probability: row.probability,
        start_offset_ms: row.start_offset_ms,
        text: row.text,
    }
}
