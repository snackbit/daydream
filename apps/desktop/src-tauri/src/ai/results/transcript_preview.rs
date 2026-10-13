use super::super::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptPreviewInput {
    pub recording_id: String,
    pub source_role: String,
    pub start_offset_ms: i64,
    pub end_offset_ms: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptPreviewEntry {
    pub id: String,
    pub segment_id: String,
    pub start_offset_ms: i64,
    pub end_offset_ms: i64,
    pub text: String,
    pub truncated: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingTranscriptPreview {
    pub recording_id: String,
    pub source_role: String,
    pub entries: Vec<TranscriptPreviewEntry>,
}

pub(crate) fn read(
    storage: &AppStorage,
    input: &TranscriptPreviewInput,
) -> Result<RecordingTranscriptPreview, AiError> {
    if input.recording_id.is_empty()
        || !matches!(input.source_role.as_str(), "microphone" | "desktop-audio")
        || input.start_offset_ms < 0
        || input.end_offset_ms <= input.start_offset_ms
    {
        return Err(ai_error(
            "ai_processing_selection_invalid",
            "Choose a valid transcript range.",
        ));
    }
    let entries = storage
        .with_connection(|connection| {
            storage::transcript_preview_entries(
                connection,
                &input.recording_id,
                &input.source_role,
                input.start_offset_ms,
                input.end_offset_ms,
            )
        })
        .map_err(storage_error)?;
    Ok(RecordingTranscriptPreview {
        recording_id: input.recording_id.clone(),
        source_role: input.source_role.clone(),
        entries: entries
            .into_iter()
            .map(|row| TranscriptPreviewEntry {
                id: row.id,
                segment_id: row.segment_id,
                start_offset_ms: row.start_offset_ms,
                end_offset_ms: row.end_offset_ms,
                text: row.text,
                truncated: row.truncated,
            })
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transcript_preview_bounds_text_and_excludes_redactions_before_limiting() {
        let storage = storage::test_storage(PathBuf::new());
        let (chunk, track) = storage
            .with_connection(|connection| {
                let (chunk, track) = crate::ai::tests::test_transcription_chunk(connection);
                storage::finalize_recording_segment(
                    connection,
                    &storage::FinalizeRecordingSegmentInput {
                        id: chunk.segment_id.clone().unwrap(),
                        duration_ms: 10_000,
                        ended_at_ms: 10_001,
                        updated_at_ms: 10_001,
                        video_stream_index: None,
                        desktop_audio_stream_index: Some(1),
                        microphone_audio_stream_index: Some(2),
                        probe_json: None,
                    },
                )?;
                storage::mark_recording_completed(connection, &chunk.recording_id, 10_001)?;
                for (index, start) in [500, 1500, 2500, 3500, 4500].into_iter().enumerate() {
                    storage::create_transcript_segment(
                        connection,
                        &storage::CreateTranscriptSegmentInput {
                            id: Uuid::from_u128(index as u128 + 1).to_string(),
                            recording_id: chunk.recording_id.clone(),
                            segment_id: chunk.segment_id.clone().unwrap(),
                            track_id: track.id.clone(),
                            source_role: "microphone".into(),
                            start_offset_ms: start,
                            end_offset_ms: start + 500,
                            provider: "local-whisper-cpp".into(),
                            model: None,
                            text: if index == 0 {
                                "🦊".repeat(241)
                            } else {
                                format!("Synthetic excerpt {index}")
                            },
                            confidence: None,
                            job_id: chunk.job_id.clone(),
                            chunk_id: chunk.id.clone(),
                            created_at_ms: 1,
                        },
                    )?;
                }
                Ok((chunk, track))
            })
            .unwrap();
        let mut input = TranscriptPreviewInput {
            recording_id: chunk.recording_id.clone(),
            source_role: "microphone".into(),
            start_offset_ms: 0,
            end_offset_ms: 10_000,
        };
        let preview = read(&storage, &input).unwrap();
        assert_eq!(preview.entries.len(), 3);
        assert_eq!(preview.entries[0].text.chars().count(), 240);
        assert!(preview.entries[0].truncated);
        assert!(!preview.entries[1].truncated);
        assert_eq!(
            preview.entries[0].segment_id,
            chunk.segment_id.clone().unwrap()
        );
        assert_eq!(
            serde_json::to_value(&preview).unwrap()["sourceRole"],
            "microphone"
        );
        storage
            .with_connection(|connection| {
                storage::create_redaction(
                    connection,
                    &storage::CreateRedactionInput {
                        id: Uuid::now_v7().to_string(),
                        recording_id: chunk.recording_id.clone(),
                        start_offset_ms: 0,
                        end_offset_ms: 2100,
                        track_ids_json: serde_json::to_string(&["microphone-2"]).unwrap(),
                        reason: None,
                        created_at_ms: 10_002,
                    },
                )
            })
            .unwrap();
        assert_eq!(
            read(&storage, &input)
                .unwrap()
                .entries
                .iter()
                .map(|entry| entry.start_offset_ms)
                .collect::<Vec<_>>(),
            [2500, 3500, 4500]
        );
        input.start_offset_ms = 3000;
        input.end_offset_ms = 3500;
        assert!(read(&storage, &input).unwrap().entries.is_empty());
        input.source_role = "desktop-audio".into();
        input.start_offset_ms = 0;
        input.end_offset_ms = 10_000;
        assert!(read(&storage, &input).unwrap().entries.is_empty());
        input.source_role = "microphone".into();
        storage
            .with_connection(|connection| {
                storage::create_redaction(
                    connection,
                    &storage::CreateRedactionInput {
                        id: Uuid::now_v7().to_string(),
                        recording_id: chunk.recording_id.clone(),
                        start_offset_ms: 0,
                        end_offset_ms: 10_000,
                        track_ids_json: "[]".into(),
                        reason: None,
                        created_at_ms: 10_003,
                    },
                )
            })
            .unwrap();
        assert!(read(&storage, &input).unwrap().entries.is_empty());
        // Legacy rows can outlive stream metadata. An all-source cut still
        // excludes them, rather than relying on synthetic lane IDs being known.
        storage.with_connection(|connection| connection.execute("UPDATE recording_segments SET microphone_audio_stream_index=NULL,desktop_audio_stream_index=NULL WHERE recording_id=?1", [&input.recording_id])).unwrap();
        assert!(read(&storage, &input).unwrap().entries.is_empty());
        for (role, from, to) in [
            ("video", 0, 1000),
            ("microphone", -1, 1000),
            ("microphone", 1000, 1000),
            ("microphone", 1001, 1000),
        ] {
            input.source_role = role.into();
            input.start_offset_ms = from;
            input.end_offset_ms = to;
            assert_eq!(
                read(&storage, &input).unwrap_err().code,
                "ai_processing_selection_invalid"
            );
        }
        input.source_role = "microphone".into();
        input.start_offset_ms = 0;
        input.end_offset_ms = 1000;
        input.recording_id.clear();
        assert!(read(&storage, &input).is_err());
        assert!(!track.id.is_empty());
    }
}
