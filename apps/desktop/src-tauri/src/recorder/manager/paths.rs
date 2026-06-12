use std::path::Path;

pub(super) fn duration_ms(started_at_ms: u64, ended_at_ms: u64) -> u64 {
    ended_at_ms.saturating_sub(started_at_ms)
}

pub(super) fn segment_file_name(segment_index: u32) -> String {
    format!("{segment_index:04}.mkv")
}

pub(super) fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().to_string()
}
