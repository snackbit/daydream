pub(crate) fn binary_path() -> String {
    std::env::var("DAYDREAM_FFMPEG_PATH").unwrap_or_else(|_| "ffmpeg".to_string())
}

/// Decode on the media timeline: retain timestamp gaps as silence without
/// stretching real samples. Apply output seeking after this filter so each
/// consumer slices the same origin, then bound padding to the requested end.
pub(crate) fn audio_timeline_filter(sample_rate: u32, end_ms: i64) -> String {
    let resample = format!("aresample={sample_rate}:async=1:first_pts=0");
    if end_ms <= 0 {
        return resample;
    }
    let end_seconds = format!("{:.3}", end_ms as f64 / 1000.);
    format!("{resample},apad=whole_dur={end_seconds},atrim=duration={end_seconds}")
}
#[cfg(test)]
#[path = "ffmpeg/audio_timeline_tests.rs"]
pub(crate) mod audio_timeline_tests;
