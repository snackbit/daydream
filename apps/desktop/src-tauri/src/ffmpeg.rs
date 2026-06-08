pub(crate) fn binary_path() -> String {
    std::env::var("DAYDREAM_FFMPEG_PATH").unwrap_or_else(|_| "ffmpeg".to_string())
}
