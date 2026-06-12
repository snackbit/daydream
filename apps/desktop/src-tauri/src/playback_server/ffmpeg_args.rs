use super::PlaybackSegment;

const THUMBNAIL_WIDTH_PX: i64 = 320;

pub(super) fn input_seek_ffmpeg_args(start_ms: i64, file_path: &str) -> Vec<String> {
    vec![
        "-i".to_string(),
        file_path.to_string(),
        "-ss".to_string(),
        format_seconds(start_ms),
    ]
}
pub(super) fn thumbnail_ffmpeg_args(
    segment: &PlaybackSegment,
    video_stream_index: i64,
    offset_ms: i64,
) -> Vec<String> {
    vec![
        "-hide_banner".to_string(),
        "-v".to_string(),
        "error".to_string(),
        "-i".to_string(),
        segment.file_path.clone(),
        "-ss".to_string(),
        format_seconds(offset_ms),
        "-map".to_string(),
        format!("0:{video_stream_index}"),
        "-frames:v".to_string(),
        "1".to_string(),
        "-an".to_string(),
        "-vf".to_string(),
        format!("scale={THUMBNAIL_WIDTH_PX}:-1,format=yuvj420p"),
        "-q:v".to_string(),
        "4".to_string(),
        "-f".to_string(),
        "image2pipe".to_string(),
        "pipe:1".to_string(),
    ]
}
fn format_seconds(ms: i64) -> String {
    format!("{:.3}", ms.max(0) as f64 / 1000.0)
}
