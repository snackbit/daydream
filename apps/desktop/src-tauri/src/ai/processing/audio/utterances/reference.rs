//! Published neighbor timestamps anchor retry output without rerunning the model.
use super::super::*;

pub(super) fn published_speech_unit(
    connection: &rusqlite::Connection,
    unit: &storage::SpeechUnit,
) -> rusqlite::Result<transcription::VerboseTranscription> {
    let mut query=connection.prepare("SELECT t.id,t.start_offset_ms,t.end_offset_ms,t.text FROM published_transcript_segments t JOIN processing_speech_unit_transcripts link ON link.transcript_id=t.id WHERE link.unit_id=?1 ORDER BY t.start_offset_ms,t.end_offset_ms,t.id")?;
    let rows = query
        .query_map([&unit.id], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, String>(3)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut segments = Vec::new();
    let mut words = Vec::new();
    for (index, (id, start, end, text)) in rows.into_iter().enumerate() {
        segments.push(transcription::VerboseTranscriptionSegment {
            id: index as i64,
            start: (start - unit.submitted_start_ms) as f64 / 1000.0,
            end: (end - unit.submitted_start_ms) as f64 / 1000.0,
            text,
        });
        for word in storage::list_transcript_words(connection, &id)? {
            words.push(transcription::VerboseTranscriptionWord {
                start: (word.start_offset_ms - unit.submitted_start_ms) as f64 / 1000.0,
                end: (word.end_offset_ms - unit.submitted_start_ms) as f64 / 1000.0,
                word: word.text,
                probability: word.probability,
            });
        }
    }
    Ok(transcription::VerboseTranscription {
        duration: Some((unit.submitted_end_ms - unit.submitted_start_ms) as f64 / 1000.0),
        language: None,
        language_confidence: None,
        task: None,
        text: segments
            .iter()
            .map(|segment| segment.text.as_str())
            .collect::<Vec<_>>()
            .join(" "),
        segments,
        words,
    })
}
