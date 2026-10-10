use super::super::media_writer::FragmentEvent;
use super::*;

/// A close can arrive in a poll before the following open, whose timestamp is
/// the authoritative cut boundary. Keep that close owned by this capture until
/// the boundary is known, or until pause/stop drains the writer.
#[derive(Debug, Default)]
pub(super) struct FragmentPersistenceQueue {
    latest_open: Option<FragmentEvent>,
    pending_closes: Vec<FragmentEvent>,
}

impl FragmentPersistenceQueue {
    pub(super) fn ready(&mut self, events: Vec<FragmentEvent>) -> Vec<FragmentEvent> {
        if let Some(open) = events
            .iter()
            .filter(|event| event.opened)
            .max_by_key(|event| event.timestamp_ms)
        {
            if self
                .latest_open
                .as_ref()
                .is_none_or(|latest| open.timestamp_ms >= latest.timestamp_ms)
            {
                self.latest_open = Some(open.clone());
            }
        }
        let events = std::mem::take(&mut self.pending_closes)
            .into_iter()
            .chain(events);
        let (ready, pending) = events.partition(|event| {
            event.opened
                || self
                    .latest_open
                    .as_ref()
                    .is_some_and(|latest| latest.path != event.path)
        });
        self.pending_closes = pending;
        ready
    }

    pub(super) fn finish(&mut self) -> Vec<FragmentEvent> {
        std::mem::take(&mut self.pending_closes)
    }
}

impl RecorderManager {
    pub(crate) fn persist_capture_fragments(
        &self,
        storage: &AppStorage,
        recording_id: &str,
    ) -> Result<(), RecorderError> {
        let active = {
            let runtime = self.lock_runtime()?;
            if status_has_pending_lifecycle(&runtime.status) {
                return Ok(());
            }
            runtime
                .active
                .as_ref()
                .filter(|active| active.recording_id == recording_id)
                .cloned()
        };
        let Some(active) = active else {
            return Ok(());
        };
        let Some(segment) = &active.active_segment else {
            return Ok(());
        };
        let Some(runtime) = &segment.runtime else {
            return Ok(());
        };
        // Serialize fragment persistence with consuming the capture runtime on
        // pause/stop. Capture callbacks only enqueue small bounded metadata.
        let mut capture = runtime.lock().map_err(|_| {
            lifecycle_error(
                "recorder_state_unavailable",
                "Capture state is unavailable.",
            )
        })?;
        let Some(capture) = capture.as_mut() else {
            return Ok(());
        };
        let events = capture.drain_fragments();
        if events.is_empty() {
            return Ok(());
        }
        let current = self.save_fragment_events(storage, &active, segment, &events)?;
        let mut owner = self.lock_runtime()?;
        owner.status.active_segment_id = Some(current.id.clone());
        if let Some(owner) = owner
            .active
            .as_mut()
            .filter(|owner| owner.recording_id == recording_id)
        {
            let index = current
                .file_path
                .file_stem()
                .and_then(|value| value.to_str())
                .and_then(|value| value.parse::<u32>().ok())
                .ok_or_else(|| {
                    lifecycle_error(
                        "recorder_fragment_path_invalid",
                        "A capture fragment has an invalid file name.",
                    )
                })?;
            owner.next_segment_index = owner.next_segment_index.max(index.saturating_add(1));
            owner.active_segment = Some(current);
        }
        Ok(())
    }

    pub(super) fn save_fragment_events(
        &self,
        storage: &AppStorage,
        active: &ActiveRecording,
        segment: &ActiveSegment,
        events: &[FragmentEvent],
    ) -> Result<ActiveSegment, RecorderError> {
        let mut current = segment.clone();
        storage
            .with_connection_mut(|connection| {
                let transaction = connection.transaction()?;
                for event in events.iter().filter(|event| event.opened) {
                    if event.path.parent() != Some(active.segments_dir.as_path()) {
                        return Err(rusqlite::Error::InvalidQuery);
                    }
                    let path = path_to_string(&event.path);
                    let rows =
                        storage::list_recording_segments(&transaction, &active.recording_id)?;
                    let row = if let Some(row) = rows.iter().find(|row| row.file_path == path)
                    {
                        row.clone()
                    } else {
                        let index = event
                            .path
                            .file_stem()
                            .and_then(|name| name.to_str())
                            .and_then(|name| name.parse::<i64>().ok())
                            .ok_or(rusqlite::Error::InvalidQuery)?;
                        storage::create_recording_segment(
                            &transaction,
                            &CreateRecordingSegmentInput {
                                file_path: path,
                                id: Uuid::now_v7().to_string(),
                                recording_id: active.recording_id.clone(),
                                segment_index: index,
                                started_at_ms: event.timestamp_ms as i64,
                            },
                        )?
                    };
                    storage::save_segment_video_streams(
                        &transaction,
                        &row.id,
                        &active.video_sources,
                        active.fps,
                    )?;
                    // An opening rollover precedes its predecessor's finalization.
                    // Use this run's durable origin, never a speculative duration
                    // from an unfinished predecessor or a legacy display fallback.
                    let base = rows
                        .iter()
                        .find(|row| row.capture_run_id.as_deref() == Some(&active.capture_run_id))
                        .filter(|origin| origin.timeline_basis_version == Some(1))
                        .and_then(|origin| {
                            let delta = row.started_at_ms.checked_sub(origin.started_at_ms)?;
                            (delta >= 0).then_some(delta).and_then(|delta| {
                                origin.timeline_base_ms?.checked_add(delta)
                            })
                        });
                    if base.is_some() && row.timeline_base_ms.is_some() && base != row.timeline_base_ms {
                        return Err(rusqlite::Error::InvalidQuery);
                    }
                    transaction.execute(
                        "UPDATE recording_segments SET capture_run_id=?2,
                         timeline_base_ms=COALESCE(timeline_base_ms,?3),
                         timeline_basis_version=CASE WHEN ?3 IS NOT NULL THEN 1 ELSE timeline_basis_version END
                         WHERE id=?1",
                        rusqlite::params![row.id, active.capture_run_id, base],
                    )?;
                    super::capture_provenance::persist_segment_provenance(
                        &transaction,
                        active,
                        segment,
                        &row.id,
                        row.started_at_ms.max(0) as u64,
                    )?;
                    if row.started_at_ms >= current.started_at_ms as i64 {
                        current.file_path = event.path.clone();
                        current.id = row.id;
                        current.started_at_ms = row.started_at_ms.max(0) as u64;
                    }
                }
                let rows = storage::list_recording_segments(&transaction, &active.recording_id)?;
                for event in events.iter().filter(|event| !event.opened) {
                    let Some(row) = rows
                        .iter()
                        .find(|row| row.file_path == path_to_string(&event.path))
                    else {
                        return Err(rusqlite::Error::QueryReturnedNoRows);
                    };
                    if row.state == "finalized" {
                        continue;
                    }
                    // The next opening is the exact cut boundary. The final fragment
                    // is finalized at the user-controlled pause/stop boundary.
                    let Some(next) = rows
                        .iter()
                        .find(|next| next.segment_index > row.segment_index)
                    else {
                        continue;
                    };
                    let duration = (next.started_at_ms - row.started_at_ms).max(0);
                    storage::finalize_recording_segment(
                        &transaction,
                        &FinalizeRecordingSegmentInput {
                            id: row.id.clone(),
                            duration_ms: duration,
                            ended_at_ms: next.started_at_ms,
                            updated_at_ms: next.started_at_ms,
                            video_stream_index: Some(segment.stream_indexes.video_stream_index),
                            desktop_audio_stream_index: segment
                                .stream_indexes
                                .desktop_audio_stream_index,
                            microphone_audio_stream_index: segment
                                .stream_indexes
                                .microphone_audio_stream_index,
                            probe_json: None,
                        },
                    )?;
                }
                transaction.commit()
            })
            .map_err(storage_error)?;
        Ok(current)
    }
}
