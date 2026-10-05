use super::super::super::*;
use super::{OcrCandidate, PreviousVisualObservation};

pub(in crate::ai) fn previous_visual_observations(
    storage: &AppStorage,
    recording_id: &str,
) -> Result<Vec<PreviousVisualObservation>, AiError> {
    let visible = visible_visual_observation_rows_for_recording(storage, recording_id)?;
    let mut observations =
        visible
            .ocr_regions
            .into_iter()
            .map(|region| PreviousVisualObservation {
                frame_timestamp_ms: region.frame_timestamp_ms,
                kind: "ocr".to_string(),
                text: region.text,
            })
            .chain(visible.vlm_observations.into_iter().map(|observation| {
                PreviousVisualObservation {
                    frame_timestamp_ms: observation.frame_timestamp_ms,
                    kind: observation.observation_kind,
                    text: observation.text,
                }
            }))
            .collect::<Vec<_>>();
    observations.sort_by(|left, right| {
        left.frame_timestamp_ms
            .cmp(&right.frame_timestamp_ms)
            .then_with(|| left.kind.cmp(&right.kind))
            .then_with(|| left.text.cmp(&right.text))
    });
    Ok(observations)
}

pub(super) fn persisted_ocr_candidates_by_timestamp(
    storage: &AppStorage,
    recording_id: &str,
) -> Result<BTreeMap<i64, Vec<OcrCandidate>>, AiError> {
    let visible = visible_visual_observation_rows_for_recording(storage, recording_id)?;
    let mut by_timestamp = BTreeMap::<i64, Vec<OcrCandidate>>::new();
    for region in visible.ocr_regions {
        by_timestamp
            .entry(region.frame_timestamp_ms)
            .or_default()
            .push(OcrCandidate {
                confidence: region.confidence,
                region: VisualRegion {
                    height: region.region_height,
                    width: region.region_width,
                    x: region.region_x,
                    y: region.region_y,
                },
                text: region.text,
            });
    }
    Ok(by_timestamp)
}

pub(super) fn recent_visual_observations(
    observations: &[PreviousVisualObservation],
    frame_timestamp_ms: i64,
) -> Vec<PreviousVisualObservation> {
    let mut recent = observations
        .iter()
        .filter(|observation| observation.frame_timestamp_ms < frame_timestamp_ms)
        .cloned()
        .collect::<Vec<_>>();
    recent.sort_by(|left, right| {
        right
            .frame_timestamp_ms
            .cmp(&left.frame_timestamp_ms)
            .then_with(|| right.kind.cmp(&left.kind))
            .then_with(|| right.text.cmp(&left.text))
    });
    recent.truncate(3);
    recent.reverse();
    recent
}
