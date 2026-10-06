use super::*;

pub(super) fn load_visual_settings(
    storage: &AppStorage,
    chunk: &ProcessingChunkRow,
) -> Result<settings::AppSettings, AiError> {
    let settings = super::super::worker::load_chunk_settings(storage, chunk)?;
    if chunk.source_kind == "vlm" && !super::vlm::vision_provider_ready(&settings) {
        return Err(ai_error("ai_vision_remote_unconfigured","Screen observations require a configured Vision endpoint, model and authentication. Review Vision settings, then retry this section."));
    }
    Ok(settings)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn review_incomplete_observation_configuration_is_an_error_for_admitted_vlm_work() {
        let storage = storage::test_storage(PathBuf::new());
        let mut chunk = ProcessingChunkRow {
            attempt_count: 1,
            created_at_ms: 1,
            end_offset_ms: Some(1000),
            error_code: None,
            id: Uuid::now_v7().to_string(),
            job_id: Uuid::now_v7().to_string(),
            progress: 0.0,
            recording_id: Uuid::now_v7().to_string(),
            result_code: None,
            segment_id: None,
            source_kind: "vlm".into(),
            start_offset_ms: Some(0),
            status: "running".into(),
            track_id: None,
            updated_at_ms: 1,
        };
        for provider in ["disabled", "openai-compatible"] {
            storage
                .with_connection(|connection| {
                    for (key, value) in [
                        ("ai.vision.provider", serde_json::json!(provider)),
                        ("ai.vision.observationsEnabled", serde_json::json!(true)),
                    ] {
                        storage::save_app_setting(
                            connection,
                            &SaveAppSettingInput {
                                key: key.into(),
                                value_json: value.to_string(),
                                is_sensitive: false,
                                timestamp_ms: 1,
                            },
                        )?;
                    }
                    Ok(())
                })
                .unwrap();
            let error = load_visual_settings(&storage, &chunk).expect_err(
                "Admitted observations must fail visibly when their profile is incomplete",
            );
            assert_eq!(error.code, "ai_vision_remote_unconfigured");
            chunk.source_kind = "ocr".into();
            assert!(
                load_visual_settings(&storage, &chunk).is_ok(),
                "Vision configuration must not disable independent OCR work"
            );
            chunk.source_kind = "vlm".into();
        }
        storage
            .with_connection(|connection| {
                for (key, value) in [
                    ("ai.vision.baseUrl", "http://127.0.0.1:9/v1"),
                    ("ai.vision.model", "synthetic"),
                    ("ai.vision.authMode", "none"),
                ] {
                    storage::save_app_setting(
                        connection,
                        &SaveAppSettingInput {
                            key: key.into(),
                            value_json: serde_json::to_string(value).unwrap(),
                            is_sensitive: false,
                            timestamp_ms: 1,
                        },
                    )?;
                }
                Ok(())
            })
            .unwrap();
        assert!(
            load_visual_settings(&storage, &chunk).is_ok(),
            "A complete anonymous profile is ready"
        );
    }
}
