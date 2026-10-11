use super::super::*;

mod chunk_input;
mod detection_context;
mod extraction;
mod language_context;
mod legacy;
mod ranges;
mod transcript_persistence;
mod utterances;

use chunk_input::load_audio_chunk_input;
use extraction::extract_audio_stream;
use ranges::{audio_path_for_speech_range, plan_submitted_transcription_ranges};
use transcript_persistence::{persist_speech_range, SpeechRangeResult};

#[cfg(test)]
pub(in crate::ai) use extraction::ffmpeg_audio_extract_args;
#[cfg(test)]
pub(in crate::ai) use transcript_persistence::{
    persist_transcriptions_for_chunk, transcript_segment_offsets_ms,
    transcription_words_for_segment, PendingTranscription,
};

pub(in crate::ai) async fn process_audio_chunk_inner(
    app: &AppHandle,
    chunk: &ProcessingChunkRow,
    cancel: Arc<AtomicBool>,
    processing_lease: Arc<tokio::sync::OwnedRwLockReadGuard<()>>,
) -> Result<AudioChunkProcessingOutcome, AiError> {
    if app
        .state::<AppStorage>()
        .with_connection(|c| storage::speech_strategy(c, &chunk.job_id))
        .map_err(storage_error)?
    {
        return utterances::process(app, chunk, cancel, processing_lease).await;
    }
    legacy::process(app, chunk, cancel, processing_lease).await
}
