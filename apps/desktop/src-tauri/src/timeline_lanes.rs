use std::collections::BTreeSet;

use crate::storage::RecordingSegmentRow;

pub(crate) const ACTIVITY_MOUSE_TRACK_ID: &str = "activity:mouse";
pub(crate) const ACTIVITY_KEYBOARD_TRACK_ID: &str = "activity:keyboard";
pub(crate) const ACTIVITY_FOCUS_TRACK_ID: &str = "activity:focus";

pub(crate) const ACTIVITY_TRACK_IDS: [&str; 3] = [
    ACTIVITY_MOUSE_TRACK_ID,
    ACTIVITY_KEYBOARD_TRACK_ID,
    ACTIVITY_FOCUS_TRACK_ID,
];

pub(crate) fn media_track_ids_from_segments(segments: &[RecordingSegmentRow]) -> Vec<String> {
    let mut track_ids = BTreeSet::new();

    for segment in segments
        .iter()
        .filter(|segment| segment.state == "finalized")
    {
        if let Some(stream_index) = segment.video_stream_index {
            track_ids.insert(format!("video-{stream_index}"));
        }
        if let Some(stream_index) = segment.desktop_audio_stream_index {
            track_ids.insert(format!("desktop-audio-{stream_index}"));
        }
        if let Some(stream_index) = segment.microphone_audio_stream_index {
            track_ids.insert(format!("microphone-{stream_index}"));
        }
    }

    track_ids.into_iter().collect()
}
