use std::collections::BTreeSet;

use crate::timeline_lanes::ACTIVITY_TRACK_IDS;

pub(crate) fn expanded_redaction_track_ids(
    track_ids_json: &str,
    media_track_ids: &[String],
) -> Vec<String> {
    let media_track_ids = normalized_track_ids(media_track_ids);
    let explicit_track_ids = parse_redaction_track_ids(track_ids_json).unwrap_or_default();
    let all_track_ids = all_track_ids(&media_track_ids);

    if explicit_track_ids.is_empty() {
        return all_track_ids;
    }

    if targets_all_media_without_activity(&explicit_track_ids, &media_track_ids) {
        return all_track_ids;
    }

    explicit_track_ids
}

pub(crate) fn parse_redaction_track_ids(track_ids_json: &str) -> Option<Vec<String>> {
    serde_json::from_str::<Vec<String>>(track_ids_json)
        .ok()
        .map(|track_ids| normalized_track_ids(&track_ids))
}

pub(crate) fn normalized_track_ids(track_ids: &[String]) -> Vec<String> {
    track_ids
        .iter()
        .map(|track_id| track_id.trim())
        .filter(|track_id| !track_id.is_empty())
        .map(ToString::to_string)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn all_track_ids(media_track_ids: &[String]) -> Vec<String> {
    media_track_ids
        .iter()
        .cloned()
        .chain(
            ACTIVITY_TRACK_IDS
                .iter()
                .map(|track_id| track_id.to_string()),
        )
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn targets_all_media_without_activity(
    explicit_track_ids: &[String],
    media_track_ids: &[String],
) -> bool {
    if media_track_ids.is_empty() {
        return false;
    }

    let explicit_track_id_set = explicit_track_ids
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let targets_activity = explicit_track_id_set
        .iter()
        .any(|track_id| ACTIVITY_TRACK_IDS.contains(track_id));

    !targets_activity
        && media_track_ids
            .iter()
            .all(|track_id| explicit_track_id_set.contains(track_id.as_str()))
}
