use super::{error, SpeechRange, WhisperBackendError};

pub(super) fn padded_range(
    start_cs: f32,
    end_cs: f32,
    duration_ms: i64,
) -> Result<SpeechRange, WhisperBackendError> {
    if !start_cs.is_finite()
        || !end_cs.is_finite()
        || start_cs < 0.0
        || end_cs <= start_cs
        || start_cs as f64 * 10.0 >= duration_ms as f64
    {
        return Err(error(
            "ai_transcription_vad_output_invalid",
            "The speech detector returned invalid timestamps.",
        ));
    }
    // The C API returns centiseconds, not seconds. Apply asymmetric context
    // here rather than using the API's symmetric speech_pad_ms parameter.
    Ok(SpeechRange {
        start_ms: ((start_cs as f64 * 10.0).floor() as i64 - 300).max(0),
        end_ms: ((end_cs as f64 * 10.0).ceil() as i64 + 150).min(duration_ms),
    })
}
