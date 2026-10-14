use super::super::super::*;
use rusqlite::OptionalExtension;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ProcessingChunkSource {
    Activity,
    Audio,
    Visual,
    Waveform,
}

impl ProcessingChunkSource {
    pub(super) fn from_chunk(chunk: &ProcessingChunkRow) -> Result<Self, AiError> {
        match chunk.source_kind.as_str() {
            "activity" => Ok(Self::Activity),
            "audio" => Ok(Self::Audio),
            "ocr" | "vlm" => Ok(Self::Visual),
            "waveform" => Ok(Self::Waveform),
            _ => Err(ai_error(
                "ai_processing_chunk_source_unsupported",
                "Processing chunk source is not supported.",
            )),
        }
    }
}

#[cfg(test)]
pub(in crate::ai) fn claim_next_processing_chunk(
    storage: &AppStorage,
    job_id: &str,
) -> Result<Option<ProcessingChunkRow>, AiError> {
    claim_with_capacity(storage, job_id, true, true)
}
pub(super) fn claim_with_capacity(
    storage: &AppStorage,
    job_id: &str,
    local: bool,
    remote: bool,
) -> Result<Option<ProcessingChunkRow>, AiError> {
    let next_chunk = storage
        .with_connection(|connection| {
            let mut chunks = storage::list_processing_chunks(connection, job_id)?;
            let chronological_visual = connection.query_row("SELECT visual_protocol='changes-v1' FROM processing_job_runtime WHERE job_id=?1", [job_id], |row| row.get::<_,bool>(0)).optional()?.unwrap_or(false);
            if let Some(chunk)=chunks.first() {
                let recording=storage::get_recording(connection,&chunk.recording_id)?;
                if !matches!(recording.status.as_str(),"recording"|"paused")&&!crate::recording_privacy::protection_ready(connection,&chunk.recording_id)?{return Err(rusqlite::Error::InvalidQuery);}
            }
            let mut running_by_kind=std::collections::HashMap::<String,usize>::new();
            for chunk in &chunks {if chunk.status=="running"{*running_by_kind.entry(chunk.source_kind.clone()).or_default()+=1;}}
            chunks.sort_by_key(|chunk|(running_by_kind.get(&chunk.source_kind).copied().unwrap_or(0),chunk.start_offset_ms.unwrap_or(0),chunk.source_kind.clone(),chunk.created_at_ms));
            for index in 0..chunks.len() {
                if chunks[index].status != "queued" {
                    continue;
                }
                if !storage::processing_chunk_retry_ready(connection, &chunks[index].id, now_ms())? { continue; }
                let chunk = &chunks[index];
                if (chunk.source_kind=="vlm" && !remote) || (chunk.source_kind!="vlm" && !local) {continue;}
                if chunk.source_kind=="audio"&&!storage::speech_strategy(connection,job_id)?&&storage::processing_job_input_open(connection,job_id)? {
                    let segments=storage::list_recording_segments(connection,&chunk.recording_id)?;
                    if let Some(index)=segments.iter().position(|segment|Some(segment.id.as_str())==chunk.segment_id.as_deref()) {
                        let segment=&segments[index];
                        let recording=storage::get_recording(connection,&chunk.recording_id)?;
                        let segment_end=segment.ended_at_ms.map(|time|time-recording.started_at_ms.unwrap_or(0));
                        if chunk.end_offset_ms==segment_end && segments.get(index+1).is_none_or(|next|next.state!="finalized") {
                            connection.execute("INSERT INTO processing_chunk_runtime(chunk_id,phase,next_retry_at_ms) VALUES(?1,'waiting-for-audio-context',?2) ON CONFLICT(chunk_id) DO UPDATE SET phase=excluded.phase,next_retry_at_ms=excluded.next_retry_at_ms",rusqlite::params![chunk.id,now_ms()+2000])?;
                            continue;
                        }
                    }
                }
                if !crate::recording_privacy::range_protection_ready(connection,&chunk.recording_id,chunk.start_offset_ms.unwrap_or(0),chunk.end_offset_ms.unwrap_or(i64::MAX))? {continue;}
                if !crate::recording_privacy::chunk_eligible(connection, chunk)? {
                    storage::delete_transcript_output_for_chunks(
                        connection,
                        std::slice::from_ref(&chunk.id),
                    )?;
                    storage::delete_activity_output_for_chunks(
                        connection,
                        std::slice::from_ref(&chunk.id),
                    )?;
                    storage::delete_visual_output_for_chunks(
                        connection,
                        std::slice::from_ref(&chunk.id),
                    )?;
                    storage::update_processing_chunk_status(
                        connection,
                        &chunk.id,
                        "succeeded",
                        1.0,
                        None,
                        now_ms(),
                    )?;
                    storage::update_processing_chunk_result_code(
                        connection,
                        &chunk.id,
                        Some("skipped-redaction"),
                        now_ms(),
                    )?;
                    chunks[index].status = "succeeded".to_string();
                    continue;
                }
                if chunk_ready_to_claim(chunk, &chunks, chronological_visual) {
                    return storage::claim_processing_chunk(connection, &chunk.id, 0.05, now_ms());
                }
            }
            Ok(None)
        })
        .map_err(storage_error)?;

    if let Some(chunk) = &next_chunk {
        tracing::info!(
            recording_id = %chunk.recording_id,
            job_id = %job_id,
            chunk_id = %chunk.id,
            source_kind = %chunk.source_kind,
            track_id = chunk.track_id.as_deref().unwrap_or(""),
            segment_id = chunk.segment_id.as_deref().unwrap_or(""),
            start_offset_ms = ?chunk.start_offset_ms,
            end_offset_ms = ?chunk.end_offset_ms,
            attempt_count = chunk.attempt_count,
            "ai_processing_chunk_claimed"
        );
    }

    Ok(next_chunk)
}

fn chunk_ready_to_claim(
    chunk: &ProcessingChunkRow,
    chunks: &[ProcessingChunkRow],
    chronological_visual: bool,
) -> bool {
    chunk_dependency_wait_reason(chunk, chunks, chronological_visual).is_none()
}

pub(crate) fn chunk_dependency_wait_reason(
    chunk: &ProcessingChunkRow,
    chunks: &[ProcessingChunkRow],
    chronological_visual: bool,
) -> Option<&'static str> {
    if (chunk.source_kind == "audio"
        || chronological_visual && matches!(chunk.source_kind.as_str(), "ocr" | "vlm"))
        && chunks.iter().any(|candidate| {
            candidate.id != chunk.id
                && candidate.source_kind == chunk.source_kind
                && candidate.track_id == chunk.track_id
                && (candidate.status == "running"
                    || candidate.status == "queued"
                        && (candidate.start_offset_ms, &candidate.id)
                            < (chunk.start_offset_ms, &chunk.id))
        })
    {
        return Some("previous-source");
    }
    if chunk.source_kind == "audio" {
        return None;
    }
    if chunk.source_kind != "vlm" {
        return None;
    }

    chunks
        .iter()
        .any(|candidate| {
            candidate.source_kind == "ocr"
                && matches!(candidate.status.as_str(), "queued" | "running")
                && candidate.segment_id == chunk.segment_id
                && (!chronological_visual || candidate.track_id == chunk.track_id)
                && chunks_overlap(candidate, chunk)
        })
        .then_some("ocr")
}

fn chunks_overlap(left: &ProcessingChunkRow, right: &ProcessingChunkRow) -> bool {
    let left_start = left.start_offset_ms.unwrap_or(0);
    let left_end = left.end_offset_ms.unwrap_or(left_start);
    let right_start = right.start_offset_ms.unwrap_or(0);
    let right_end = right.end_offset_ms.unwrap_or(right_start);

    left_start < right_end && right_start < left_end
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audio_waits_for_previous_speech_on_its_track_but_not_other_tracks() {
        let segment = "synthetic-segment";
        let mut first = test_chunk("audio", "running", 0, 1000, segment);
        first.track_id = Some("mic".into());
        let next = ProcessingChunkRow {
            id: "later".into(),
            status: "queued".into(),
            start_offset_ms: Some(1000),
            end_offset_ms: Some(2000),
            ..first.clone()
        };
        assert!(!chunk_ready_to_claim(
            &next,
            &[first.clone(), next.clone()],
            false
        ));
        let mut desktop = next.clone();
        desktop.track_id = Some("desktop".into());
        assert!(chunk_ready_to_claim(
            &desktop,
            &[first.clone(), desktop.clone()],
            false
        ));
        first.status = "queued".into();
        assert!(!chunk_ready_to_claim(
            &next,
            &[next.clone(), first.clone()],
            false
        ));
        first.status = "failed".into();
        assert!(chunk_ready_to_claim(&next, &[first, next.clone()], false));
    }

    #[test]
    fn vlm_chunk_waits_for_overlapping_ocr_chunk() {
        let segment_id = Uuid::now_v7().to_string();
        let ocr = test_chunk("ocr", "running", 0, 1_000, &segment_id);
        let vlm = test_chunk("vlm", "queued", 0, 1_000, &segment_id);

        assert!(!chunk_ready_to_claim(
            &vlm,
            &[ocr.clone(), vlm.clone()],
            false
        ));

        let succeeded_ocr = ProcessingChunkRow {
            status: "succeeded".to_string(),
            ..ocr
        };
        assert!(chunk_ready_to_claim(
            &vlm,
            &[succeeded_ocr, vlm.clone()],
            false
        ));
    }

    #[test]
    fn failed_ocr_does_not_strand_independent_observations() {
        let segment_id = Uuid::now_v7().to_string();
        let ocr = test_chunk("ocr", "failed", 0, 1_000, &segment_id);
        let vlm = test_chunk("vlm", "queued", 0, 1_000, &segment_id);
        assert!(chunk_ready_to_claim(&vlm, &[ocr, vlm.clone()], false));
    }

    #[test]
    fn studio_comparison_orders_each_source_without_blocking_other_sources_or_legacy_work() {
        let first = test_chunk("vlm", "running", 0, 1000, "first");
        let next = test_chunk("vlm", "queued", 1000, 2000, "next");
        assert!(!chunk_ready_to_claim(
            &next,
            &[first.clone(), next.clone()],
            true
        ));
        assert!(chunk_ready_to_claim(
            &next,
            &[first.clone(), next.clone()],
            false
        ));
        let independent = ProcessingChunkRow {
            track_id: Some("other-screen".into()),
            ..next.clone()
        };
        assert!(chunk_ready_to_claim(
            &independent,
            &[first, independent.clone()],
            true
        ));
        let ocr = ProcessingChunkRow {
            track_id: Some("other-screen".into()),
            ..test_chunk("ocr", "running", 1000, 2000, "next")
        };
        assert!(chunk_ready_to_claim(
            &next,
            &[ocr.clone(), next.clone()],
            true
        ));
        assert!(!chunk_ready_to_claim(&next, &[ocr, next.clone()], false));
    }

    fn test_chunk(
        source_kind: &str,
        status: &str,
        start_offset_ms: i64,
        end_offset_ms: i64,
        segment_id: &str,
    ) -> ProcessingChunkRow {
        ProcessingChunkRow {
            attempt_count: 0,
            created_at_ms: 1,
            end_offset_ms: Some(end_offset_ms),
            error_code: None,
            id: Uuid::now_v7().to_string(),
            job_id: Uuid::now_v7().to_string(),
            progress: 0.0,
            recording_id: Uuid::now_v7().to_string(),
            result_code: None,
            segment_id: Some(segment_id.to_string()),
            source_kind: source_kind.to_string(),
            start_offset_ms: Some(start_offset_ms),
            status: status.to_string(),
            track_id: None,
            updated_at_ms: 1,
        }
    }
}
