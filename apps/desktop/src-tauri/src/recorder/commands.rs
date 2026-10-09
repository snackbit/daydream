mod attachments;
mod automatic;
mod completion;
mod lifecycle_commands;
mod preview_commands;
mod source_commands;
mod status_commands;
mod supervision;
pub(crate) use supervision::report_recording_failure;

pub(crate) const RECORDER_STATUS_CHANGED_EVENT: &str = "recorder-status-changed";
pub(crate) const RECORDER_PREVIEW_FRAME_EVENT: &str = "recorder-preview-frame";

pub(crate) use automatic::{request_auto_pause, request_auto_resume};
pub(crate) use lifecycle_commands::{
    pause_recording, resume_recording, start_recording, stop_recording,
};
#[cfg(test)]
pub(crate) use preview_commands::stop_empty_preview_start;
pub(crate) use preview_commands::{
    get_recorder_preview_status, start_recorder_preview, stop_recorder_preview,
    update_recorder_preview_demand,
};
pub(crate) use source_commands::{
    get_audio_sources, get_capture_sources, get_recorder_scene, save_recorder_scene,
    select_capture_sources,
};
#[cfg(test)]
pub(crate) use status_commands::collect_recorder_recording_stats;
pub(crate) use status_commands::{
    get_recorder_performance_stats, get_recorder_recording_stats, get_recorder_status,
    get_window_protection_status, list_protectable_windows,
};
