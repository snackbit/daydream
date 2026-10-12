use super::super::*;

#[derive(Clone)]
pub(super) struct AudioPart {
    pub segment: RecordingSegmentRow,
    pub stream: i64,
    pub base: i64,
    pub start: i64,
    pub end: i64,
}
#[derive(Clone)]
pub(super) struct SourceRun {
    pub parts: Vec<AudioPart>,
    pub start: i64,
    pub end: i64,
    pub open: bool,
    pub cleanup: Option<std::sync::Arc<crate::library::audio_cleanup::CleanupManifest>>,
}
impl SourceRun {
    pub(super) fn window(&self, start: i64, end: i64) -> Self {
        let first = self.parts.partition_point(|part| part.end <= start);
        Self {
            parts: self.parts[first..]
                .iter()
                .take_while(|part| part.start < end)
                .cloned()
                .collect(),
            start,
            end,
            open: false,
            cleanup: self.cleanup.clone(),
        }
    }
}

pub(super) fn source_runs(
    storage: &AppStorage,
    chunk: &ProcessingChunkRow,
    track: &RecordingTrackRow,
) -> Result<Vec<SourceRun>, AiError> {
    storage
        .with_connection(|connection| {
            let cleanup = if track.kind == "microphone" {
                crate::library::audio_cleanup::pinned_manifest(
                    connection,
                    &chunk.job_id,
                    &chunk.recording_id,
                )?
                .map(std::sync::Arc::new)
            } else {
                None
            };
            let segments = storage::list_recording_segments(connection, &chunk.recording_id)?;
            let bases = storage::segment_playback_bases(&segments);
            let redactions = storage::list_redactions(connection, &chunk.recording_id)?;
            let chunks = storage::list_processing_chunks(connection, &chunk.job_id)?;
            let mut ranges = chunks
                .iter()
                .filter(|child| {
                    child.source_kind == "audio" && child.track_id.as_ref() == Some(&track.id)
                })
                .filter_map(|child| Some((child.start_offset_ms?, child.end_offset_ms?)))
                .filter(|(start, end)| end > start)
                .collect::<Vec<_>>();
            ranges.sort_unstable();
            let mut owned: Vec<(i64, i64)> = Vec::new();
            for (start, end) in ranges {
                if let Some(last) = owned.last_mut() {
                    if start <= last.1 {
                        last.1 = last.1.max(end);
                        continue;
                    }
                }
                owned.push((start, end));
            }
            let recording = storage::get_recording(connection, &chunk.recording_id)?;
            let input_open = storage::processing_job_input_open(connection, &chunk.job_id)?
                && recording.status == "recording";
            let mut parts = Vec::new();
            for (segment, base) in segments.iter().zip(bases) {
                if segment.state != "finalized" || !Path::new(&segment.file_path).is_file() {
                    continue;
                }
                let Some(stream) = (if track.kind == "microphone" {
                    segment.microphone_audio_stream_index
                } else {
                    segment.desktop_audio_stream_index
                }) else {
                    continue;
                };
                let end = base + segment.duration_ms.unwrap_or(0);
                let Some(source) = crate::recording_privacy::source_track_id(segment, &track.kind)
                else {
                    continue;
                };
                let ids = [source, track.id.clone()];
                let first = owned.partition_point(|(_, end)| *end <= base);
                for &(from, to) in owned[first..].iter().take_while(|(start, _)| *start < end) {
                    let start = base.max(from);
                    let end = end.min(to);
                    for (start, end) in crate::ai::processing::planning::visible_ranges(
                        start,
                        end,
                        &redactions,
                        Some(&ids[0]),
                    ) {
                        if end <= start
                            || crate::recording_privacy::range_redacted(
                                &redactions,
                                &ids,
                                start,
                                end,
                            )
                            || !crate::recording_privacy::range_protection_ready(
                                connection,
                                &chunk.recording_id,
                                start,
                                end,
                            )?
                        {
                            continue;
                        }
                        parts.push(AudioPart {
                            segment: segment.clone(),
                            stream,
                            base,
                            start,
                            end,
                        });
                    }
                }
            }
            parts.sort_by_key(|part| part.start);
            parts.dedup_by(|a, b| {
                a.start == b.start && a.end == b.end && a.segment.id == b.segment.id
            });
            let last_end = parts.last().map(|part| part.end);
            let mut runs: Vec<SourceRun> = Vec::new();
            for part in parts {
                if let Some(run) = runs.last_mut() {
                    let contiguous = run.parts.last().is_some_and(|previous| {
                        previous.segment.id == part.segment.id
                            || previous.segment.ended_at_ms == Some(part.segment.started_at_ms)
                    });
                    if part.start <= run.end && contiguous {
                        run.end = run.end.max(part.end);
                        run.parts.push(part);
                        continue;
                    }
                }
                runs.push(SourceRun {
                    start: part.start,
                    end: part.end,
                    parts: vec![part],
                    open: false,
                    cleanup: cleanup.clone(),
                });
            }
            if let Some(run) = runs.last_mut() {
                run.open = input_open && Some(run.end) == last_end;
            }
            Ok(runs)
        })
        .map_err(storage_error)
}

/// Extract only eligible slices of a source into one bounded mono PCM window.
pub(super) fn extract_window(
    run: &SourceRun,
    start: i64,
    end: i64,
    path: &Path,
    cancel: &AtomicBool,
) -> Result<(), AiError> {
    if start < run.start || end > run.end || end <= start || end - start > 30_000 {
        return Err(ai_error(
            "ai_processing_audio_range_invalid",
            "The speech window is invalid.",
        ));
    }
    let mut samples = Vec::with_capacity((end - start) as usize * 16);
    let mut cursor = start;
    for (index, part) in run.parts.iter().enumerate() {
        let from = cursor.max(part.start);
        let to = end.min(part.end);
        if to <= from {
            continue;
        }
        if from != cursor {
            return Err(ai_error(
                "ai_processing_audio_range_invalid",
                "Audio context is not continuous.",
            ));
        }
        check_atomic_cancel(cancel)?;
        let temp = TempAudioFile::new(path.with_extension(format!("part-{index}.wav")));
        if let Some(cleanup) = &run.cleanup {
            crate::library::audio_cleanup::media::extract_cleaned_window(
                cleanup,
                &part.segment,
                from - part.base,
                to - from,
                temp.path(),
                cancel,
            )
            .map_err(|_| {
                ai_error(
                    "ai_processing_audio_cleanup_unavailable",
                    "Cleaned audio is unavailable. Retry cleanup before transcription.",
                )
            })?;
        } else {
            extract_audio_stream(
                &part.segment.file_path,
                part.stream,
                from - part.base,
                to - from,
                temp.path(),
                cancel,
            )?;
        }
        let mut extracted = whisper_backend::vad::read_detection_samples(temp.path())
            .map_err(whisper_backend_error)?;
        let count = (to - from) as usize * 16;
        // Decoder rounding must not shift the next media fragment's timestamps.
        extracted.resize(count, 0.0);
        samples.extend(extracted.into_iter().take(count));
        cursor = to;
        if cursor == end {
            break;
        }
    }
    if cursor != end {
        return Err(ai_error(
            "ai_processing_audio_range_invalid",
            "Audio context is incomplete.",
        ));
    }
    whisper_backend::vad::write_detection_samples(path, &samples).map_err(whisper_backend_error)
}
