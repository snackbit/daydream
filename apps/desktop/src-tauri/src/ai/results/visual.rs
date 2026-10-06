use super::super::*;
use super::vlm::vlm_state_for_recording;

pub(crate) fn visual_observations_for_recording(
    storage: &AppStorage,
    recording_id: &str,
) -> Result<RecordingVisualObservations, AiError> {
    visual_observations_for_recording_with_regions(storage, recording_id, true)
}
pub(crate) fn visual_observations_for_recording_with_regions(
    storage: &AppStorage,
    recording_id: &str,
    include_ocr_regions: bool,
) -> Result<RecordingVisualObservations, AiError> {
    let app_settings = settings::load_app_settings(storage).map_err(settings_error)?;
    let visible_rows = visible_visual_rows(storage, recording_id, include_ocr_regions)?;
    let vlm_chunks = visible_rows
        .visual_chunks
        .iter()
        .filter(|chunk| chunk.source_kind == "vlm")
        .cloned()
        .collect::<Vec<_>>();
    let vlm_state = vlm_state_for_recording(
        &app_settings,
        &vlm_chunks,
        visible_rows.vlm_observations.len(),
    );

    Ok(RecordingVisualObservations {
        ocr_regions: visible_rows
            .ocr_regions
            .into_iter()
            .map(ocr_region_from_row)
            .collect(),
        recording_id: recording_id.to_string(),
        vlm_state,
        vlm_observations: visible_rows
            .vlm_observations
            .into_iter()
            .map(vlm_observation_from_row)
            .collect(),
    })
}

pub(crate) struct VisibleVisualObservationRows {
    pub(crate) ocr_regions: Vec<OcrRegionRow>,
    pub(crate) vlm_observations: Vec<VlmObservationRow>,
    visual_chunks: Vec<ProcessingChunkRow>,
}

pub(crate) fn visible_visual_observation_rows_for_recording(
    storage: &AppStorage,
    recording_id: &str,
) -> Result<VisibleVisualObservationRows, AiError> {
    visible_visual_rows(storage, recording_id, true)
}
fn visible_visual_rows(
    storage: &AppStorage,
    recording_id: &str,
    include_ocr_regions: bool,
) -> Result<VisibleVisualObservationRows, AiError> {
    let (ocr_regions, vlm_observations, visual_chunks, recording_tracks, redactions, segments,committed_frames) =
        storage
            .with_connection(|connection| {
                let mut statement=connection.prepare("SELECT f.chunk_id,f.frame_timestamp_ms FROM visual_frame_processing_states f JOIN processing_chunks c ON c.id=f.chunk_id JOIN processing_jobs j ON j.id=c.job_id WHERE f.recording_id=?1 AND f.status='succeeded' AND c.status='running' AND j.status='running'")?;
                let mut committed=BTreeMap::<String,BTreeSet<i64>>::new();
                for row in statement.query_map([recording_id],|row|Ok((row.get::<_,String>(0)?,row.get::<_,i64>(1)?)))? {let (chunk,timestamp)=row?;committed.entry(chunk).or_default().insert(timestamp);}
                Ok((
                    if include_ocr_regions {
                        storage::list_ocr_regions(connection, recording_id)?
                    } else {
                        Vec::new()
                    },
                    storage::list_vlm_observations(connection, recording_id)?,
                    list_visual_chunks_for_recording(connection, recording_id)?,
                    storage::list_recording_tracks(connection, recording_id)?,
                    storage::list_redactions(connection, recording_id)?,
                    storage::list_recording_segments(connection, recording_id)?,
                    committed,
                ))
            })
            .map_err(storage_error)?;
    let visible_ocr_regions = filter_visible_ocr_regions(
        ocr_regions,
        &visual_chunks,
        &recording_tracks,
        &redactions,
        &segments,
        &committed_frames,
    );
    let visible_vlm_observations = filter_visible_vlm_observations(
        vlm_observations,
        &visual_chunks,
        &recording_tracks,
        &redactions,
        &segments,
        &committed_frames,
    );

    Ok(VisibleVisualObservationRows {
        ocr_regions: visible_ocr_regions,
        visual_chunks,
        vlm_observations: visible_vlm_observations,
    })
}

fn list_visual_chunks_for_recording(
    connection: &rusqlite::Connection,
    recording_id: &str,
) -> rusqlite::Result<Vec<ProcessingChunkRow>> {
    let mut statement = connection.prepare(
        r#"
        SELECT
            id,
            job_id,
            recording_id,
            source_kind,
            segment_id,
            start_offset_ms,
            end_offset_ms,
            status,
            attempt_count,
            progress,
            error_code,
            result_code,
            track_id,
            created_at_ms,
            updated_at_ms
        FROM processing_chunks
        WHERE recording_id = ?1
            AND source_kind IN ('ocr', 'vlm')
        ORDER BY source_kind ASC, start_offset_ms ASC, id ASC
        "#,
    )?;
    let rows = statement.query_map([recording_id], |row| {
        Ok(ProcessingChunkRow {
            id: row.get(0)?,
            job_id: row.get(1)?,
            recording_id: row.get(2)?,
            source_kind: row.get(3)?,
            segment_id: row.get(4)?,
            start_offset_ms: row.get(5)?,
            end_offset_ms: row.get(6)?,
            status: row.get(7)?,
            attempt_count: row.get(8)?,
            progress: row.get(9)?,
            error_code: row.get(10)?,
            result_code: row.get(11)?,
            track_id: row.get(12)?,
            created_at_ms: row.get(13)?,
            updated_at_ms: row.get(14)?,
        })
    })?;

    rows.collect()
}

fn filter_visible_ocr_regions(
    regions: Vec<OcrRegionRow>,
    visual_chunks: &[ProcessingChunkRow],
    recording_tracks: &[RecordingTrackRow],
    redactions: &[RedactionRow],
    segments: &[RecordingSegmentRow],
    committed_frames: &BTreeMap<String, BTreeSet<i64>>,
) -> Vec<OcrRegionRow> {
    let visible = VisualObservationVisibility::new(
        visual_chunks,
        recording_tracks,
        redactions,
        segments,
        committed_frames,
    );

    regions
        .into_iter()
        .filter(|region| visible.is_visible(&region.chunk_id, region.frame_timestamp_ms))
        .collect()
}

fn filter_visible_vlm_observations(
    observations: Vec<VlmObservationRow>,
    visual_chunks: &[ProcessingChunkRow],
    recording_tracks: &[RecordingTrackRow],
    redactions: &[RedactionRow],
    segments: &[RecordingSegmentRow],
    committed_frames: &BTreeMap<String, BTreeSet<i64>>,
) -> Vec<VlmObservationRow> {
    let visible = VisualObservationVisibility::new(
        visual_chunks,
        recording_tracks,
        redactions,
        segments,
        committed_frames,
    );

    observations
        .into_iter()
        .filter(|observation| {
            visible.is_visible(&observation.chunk_id, observation.frame_timestamp_ms)
        })
        .collect()
}

struct VisualObservationVisibility<'a> {
    committed_frames: &'a BTreeMap<String, BTreeSet<i64>>,
    redactions: &'a [RedactionRow],
    succeeded_chunk_ids: HashSet<&'a str>,
    video_track_ids: BTreeSet<&'a str>,
    library_video_tracks_by_chunk: BTreeMap<&'a str, String>,
}

impl<'a> VisualObservationVisibility<'a> {
    fn new(
        visual_chunks: &'a [ProcessingChunkRow],
        recording_tracks: &'a [RecordingTrackRow],
        redactions: &'a [RedactionRow],
        segments: &[RecordingSegmentRow],
        committed_frames: &'a BTreeMap<String, BTreeSet<i64>>,
    ) -> Self {
        Self {
            committed_frames,
            redactions,
            succeeded_chunk_ids: visual_chunks
                .iter()
                .filter(|chunk| {
                    matches!(chunk.status.as_str(), "succeeded" | "completed-with-errors")
                })
                .map(|chunk| chunk.id.as_str())
                .collect(),
            video_track_ids: recording_tracks
                .iter()
                .filter(|track| matches!(track.kind.as_str(), "screen" | "video"))
                .map(|track| track.id.as_str())
                .collect(),
            library_video_tracks_by_chunk: visual_chunks
                .iter()
                .filter_map(|chunk| {
                    let segment = segments
                        .iter()
                        .find(|segment| Some(segment.id.as_str()) == chunk.segment_id.as_deref())?;
                    Some((
                        chunk.id.as_str(),
                        chunk.track_id.clone().or_else(|| {
                            crate::recording_privacy::source_track_id(segment, "video")
                        })?,
                    ))
                })
                .collect(),
        }
    }

    fn is_visible(&self, chunk_id: &str, frame_timestamp_ms: i64) -> bool {
        (self.succeeded_chunk_ids.contains(chunk_id)
            || self
                .committed_frames
                .get(chunk_id)
                .is_some_and(|frames| frames.contains(&frame_timestamp_ms)))
            && !self.redactions.iter().any(|redaction| {
                self.redaction_hides_visual_timestamp(redaction, chunk_id, frame_timestamp_ms)
            })
    }

    fn redaction_hides_visual_timestamp(
        &self,
        redaction: &RedactionRow,
        chunk_id: &str,
        frame_timestamp_ms: i64,
    ) -> bool {
        if frame_timestamp_ms < redaction.start_offset_ms
            || frame_timestamp_ms >= redaction.end_offset_ms
        {
            return false;
        }

        let redacted_track_ids = redaction_track_ids(redaction);
        if redacted_track_ids.is_empty() {
            return true;
        }

        redacted_track_ids.iter().any(|track_id| {
            (self
                .library_video_tracks_by_chunk
                .get(chunk_id)
                .is_none_or(|id| id.starts_with("video-"))
                && self.video_track_ids.contains(track_id.as_str()))
                || self.library_video_tracks_by_chunk.get(chunk_id) == Some(track_id)
        })
    }
}

fn redaction_track_ids(redaction: &RedactionRow) -> Vec<String> {
    serde_json::from_str::<Vec<String>>(&redaction.track_ids_json).unwrap_or_default()
}

fn visual_region_from_parts(x: i64, y: i64, width: i64, height: i64) -> VisualRegion {
    VisualRegion {
        height,
        width,
        x,
        y,
    }
}

fn ocr_region_from_row(row: OcrRegionRow) -> OcrRegion {
    OcrRegion {
        chunk_id: row.chunk_id,
        confidence: row.confidence,
        frame_timestamp_ms: row.frame_timestamp_ms,
        id: row.id,
        job_id: row.job_id,
        model: row.model,
        provider: row.provider,
        recording_id: row.recording_id,
        region: visual_region_from_parts(
            row.region_x,
            row.region_y,
            row.region_width,
            row.region_height,
        ),
        source_frame_ref: row.source_frame_ref,
        text: row.text,
    }
}

fn vlm_observation_from_row(row: VlmObservationRow) -> VlmObservation {
    VlmObservation {
        chunk_id: row.chunk_id,
        confidence: row.confidence,
        frame_timestamp_ms: row.frame_timestamp_ms,
        id: row.id,
        job_id: row.job_id,
        model: row.model,
        observation_kind: row.observation_kind,
        prompt_version: row.prompt_version,
        provider: row.provider,
        recording_id: row.recording_id,
        region: visual_region_from_parts(
            row.region_x,
            row.region_y,
            row.region_width,
            row.region_height,
        ),
        source_frame_ref: row.source_frame_ref,
        text: row.text,
    }
}
