use super::super::results::transcript_preview::{
    RecordingTranscriptPreview, TranscriptPreviewInput,
};
use super::super::*;

#[tauri::command(rename = "getRecordingTranscriptPreview")]
pub async fn get_recording_transcript_preview(
    input: TranscriptPreviewInput,
    storage: State<'_, AppStorage>,
) -> Result<RecordingTranscriptPreview, AiError> {
    let id = input.recording_id.clone();
    read_recording(storage.inner().clone(), id, move |storage| {
        super::super::results::transcript_preview::read(storage, &input)
    })
    .await
}

#[tauri::command(rename = "getRecordingAiProcessingState")]
pub async fn get_recording_ai_processing_state(
    input: RecordingAiResultInput,
    storage: State<'_, AppStorage>,
) -> Result<RecordingAiProcessingState, AiError> {
    let id = input.recording_id.clone();
    let storage = storage.inner().clone();
    read_recording(storage, id, move |storage| {
        let range = result_range(&input)?;
        let review = review_range(&input)?;
        if let Some(bucket) = input.bucket_ms {
            let (start, end) = range.ok_or_else(|| {
                ai_error(
                    "ai_processing_selection_invalid",
                    "Choose an overview time range.",
                )
            })?;
            let detail = review.or(Some((start, start.saturating_add(300_000).min(end))));
            let mut state = super::super::results::processing_state_in_range(
                storage,
                &input.recording_id,
                detail,
            )?;
            state.timeline_overview = Some(super::super::results::studio_overview::read(
                storage,
                &input.recording_id,
                start,
                end,
                bucket,
            )?);
            state.visual_frames.retain(|frame| frame.status == "failed");
            return Ok(state);
        }
        match range {
            None => processing_state_for_recording(storage, &input.recording_id),
            range => super::super::results::processing_state_with_review(
                storage,
                &input.recording_id,
                range,
                review,
            ),
        }
    })
    .await
}

#[tauri::command(rename = "getRecordingTranscript")]
pub async fn get_recording_transcript(
    input: GetRecordingTranscriptInput,
    storage: State<'_, AppStorage>,
) -> Result<RecordingTranscript, AiError> {
    let id = input.recording_id.clone();
    let storage = storage.inner().clone();
    read_recording(storage, id, move |storage| {
        transcript_for_recording(storage, &input.recording_id)
    })
    .await
}

#[tauri::command(rename = "getRecordingActivity")]
pub async fn get_recording_activity(
    input: RecordingAiResultInput,
    storage: State<'_, AppStorage>,
) -> Result<RecordingActivity, AiError> {
    let id = input.recording_id.clone();
    let storage = storage.inner().clone();
    read_recording(storage, id, move |storage| {
        let range = content_range(&input)?;
        let review = review_range(&input)?;
        super::super::results::activity_for_recording_with_review(
            storage,
            &input.recording_id,
            range,
            review,
        )
    })
    .await
}

#[tauri::command(rename = "getRecordingVisualObservations")]
pub async fn get_recording_visual_observations(
    input: RecordingAiResultInput,
    storage: State<'_, AppStorage>,
) -> Result<RecordingVisualObservations, AiError> {
    let id = input.recording_id.clone();
    let storage = storage.inner().clone();
    read_recording(storage, id, move |storage| {
        let range = content_range(&input)?;
        let review = review_range(&input)?;
        let mut result = visual_observations_for_recording_with_regions(
            storage,
            &input.recording_id,
            input.include_ocr_regions.unwrap_or(true),
        )?;
        if let Some((start, end)) = range {
            result.ocr_regions.retain(|region| {
                region.frame_timestamp_ms >= start && region.frame_timestamp_ms < end
            });
            result.vlm_observations.retain(|observation| {
                (observation.frame_timestamp_ms < end
                    && observation
                        .confirmed_end_ms
                        .unwrap_or(observation.frame_timestamp_ms + 1)
                        > start)
                    || review.is_some_and(|(from, to)| {
                        observation.frame_timestamp_ms < to
                            && observation
                                .confirmed_end_ms
                                .unwrap_or(observation.frame_timestamp_ms + 1)
                                > from
                    })
            });
        }
        Ok(result)
    })
    .await
}

fn result_range(input: &RecordingAiResultInput) -> Result<Option<(i64, i64)>, AiError> {
    match (input.start_offset_ms, input.end_offset_ms) {
        (None, None) => Ok(None),
        (Some(start), Some(end)) if start >= 0 && end > start => Ok(Some((start, end))),
        _ => Err(ai_error(
            "ai_processing_selection_invalid",
            "Choose a valid Studio time range.",
        )),
    }
}
fn review_range(input: &RecordingAiResultInput) -> Result<Option<(i64, i64)>, AiError> {
    match (input.review_start_offset_ms, input.review_end_offset_ms) {
        (None, None) => Ok(None),
        (Some(start), Some(end)) if start >= 0 && end > start && end - start <= 600_000 => {
            Ok(Some((start, end)))
        }
        _ => Err(ai_error(
            "ai_processing_selection_invalid",
            "Choose a valid review page.",
        )),
    }
}
fn content_range(input: &RecordingAiResultInput) -> Result<Option<(i64, i64)>, AiError> {
    let range = result_range(input)?;
    if input.bucket_ms.is_some() {
        Ok(review_range(input)?
            .or(range.map(|(start, end)| (start, start.saturating_add(300_000).min(end)))))
    } else {
        Ok(range)
    }
}

async fn read_recording<Result: Send + 'static>(
    storage: AppStorage,
    id: String,
    read: impl FnOnce(&AppStorage) -> std::result::Result<Result, AiError> + Send + 'static,
) -> std::result::Result<Result, AiError> {
    let lease = crate::recording_privacy::processing_lease(&id).await;
    tauri::async_runtime::spawn_blocking(move || {
        let _lease = lease;
        read(&storage)
    })
    .await
    .map_err(|_| {
        ai_error(
            "ai_processing_worker_failed",
            "Studio results could not be loaded. Try refreshing.",
        )
    })?
}
