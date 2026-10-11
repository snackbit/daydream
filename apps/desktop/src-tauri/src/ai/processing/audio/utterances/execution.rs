use super::*;

pub(super) struct SpeechWorker<'a> {
    pub app: &'a AppHandle,
    pub chunk: &'a ProcessingChunkRow,
    pub track: &'a RecordingTrackRow,
    pub run: &'a SourceRun,
    pub settings: &'a settings::AppSettings,
    pub fingerprint: &'a str,
    pub cancel: Arc<AtomicBool>,
    pub lease: crate::recording_privacy::ProcessingLease,
}

impl SpeechWorker<'_> {
    pub(super) async fn process_units(&self, target: i64) -> Result<(), AiError> {
        let (app, chunk, track, run, settings, cancel) = (
            self.app,
            self.chunk,
            self.track,
            self.run,
            self.settings,
            &self.cancel,
        );
        let storage = app.state::<AppStorage>();
        let units = storage
            .with_connection(|c| storage::speech_units(c, &chunk.job_id, &track.id))
            .map_err(storage_error)?;
        let selection = super::super::super::worker::load_unit_selection(&storage, &chunk.id)?;
        let mut previous: Option<(storage::SpeechUnit, transcription::VerboseTranscription)> = None;
        for mut unit in units.into_iter().filter(|unit| {
            unit.start_ms >= run.start
                && unit.end_ms <= run.end
                && unit.start_ms < target
                && unit.status != "published"
        }) {
            if unit.status == "failed"
                && selection.audio_ranges.as_ref().is_some_and(|ranges| {
                    !ranges.iter().any(|(start, end)| {
                        unit.start_ms < chunk.start_offset_ms.unwrap_or(0) + end
                            && unit.end_ms > chunk.start_offset_ms.unwrap_or(0) + start
                    })
                })
            {
                previous = None;
                continue;
            }
            if unit.status == "failed" && unit.owner_chunk_id != chunk.id {
                previous = None;
                if unit.end_ms > chunk.start_offset_ms.unwrap_or(0) {
                    return Err(ai_error(
                        "ai_processing_speech_unit_failed",
                        "Retry the earlier failed speech section before continuing this section.",
                    ));
                }
                continue;
            }
            check_atomic_cancel(cancel)?;
            ensure_chunk_active(&storage, chunk)?;
            if unit.fingerprint != self.fingerprint {
                unit.fingerprint = self.fingerprint.into();
                unit.output_json = None;
                storage.with_connection(|c|{
                c.execute("UPDATE processing_speech_units SET config_fingerprint=?2,output_json=NULL,status='queued',error_code=NULL WHERE id=?1 AND status!='published'",rusqlite::params![unit.id,unit.fingerprint])?;Ok(())
            }).map_err(storage_error)?;
            }
            let mut value = if let Some(json) = &unit.output_json {
                serde_json::from_str(json).map_err(|_| {
                    ai_error(
                        "ai_processing_config_invalid",
                        "The saved speech checkpoint is invalid.",
                    )
                })?
            } else if unit.submitted_end_ms == unit.submitted_start_ms {
                empty_transcription()
            } else {
                let staged_language = previous.as_ref().and_then(|(before, value)| {
                    if before.fingerprint != unit.fingerprint {
                        return None;
                    }
                    language_context::result_context(
                        &settings.ai.transcription.language,
                        None,
                        value,
                        &before.fingerprint,
                        before.submitted_start_ms,
                        before.submitted_end_ms,
                    )
                });
                match self.transcribe_unit(&unit, staged_language).await {
                    Ok(value) => value,
                    Err(error) => {
                        storage
                            .with_connection(|c| {
                                storage::fail_speech_unit(c, &unit.id, &error.code)
                            })
                            .map_err(storage_error)?;
                        return Err(error);
                    }
                }
            };
            if let Some((before, mut left)) = previous.take() {
                stitching::reconcile(
                    &mut left,
                    before.submitted_start_ms,
                    &mut value,
                    unit.submitted_start_ms,
                    before.submitted_start_ms.max(unit.submitted_start_ms),
                    before.submitted_end_ms.min(unit.submitted_end_ms),
                );
                publication::publish(&storage, chunk, track, settings, &before, left)?;
            }
            let json = serde_json::to_string(&value).map_err(|_| {
                ai_error(
                    "ai_processing_config_invalid",
                    "The speech checkpoint could not be saved.",
                )
            })?;
            publication::checkpoint(&storage, chunk, |c| {
                storage::stage_speech_unit(c, &unit.id, &json)?;
                Ok(true)
            })?;
            if unit.right_open {
                previous = Some((unit, value));
            } else {
                publication::publish(&storage, chunk, track, settings, &unit, value)?;
            }
            emit_processing_state(app, &chunk.recording_id);
        }
        // Staged boundary output stays hidden until its neighbor can be reconciled.
        if let Some((unit, value)) = previous {
            if publication::finish_with_published_neighbor(
                &storage, chunk, track, settings, &unit, value,
            )? {
                emit_processing_state(app, &chunk.recording_id);
            }
        }
        Ok(())
    }

    async fn transcribe_unit(
        &self,
        unit: &storage::SpeechUnit,
        staged_language: Option<storage::TranscriptLanguageContext>,
    ) -> Result<transcription::VerboseTranscription, AiError> {
        let (app, chunk, track, run, settings, cancel, lease) = (
            self.app,
            self.chunk,
            self.track,
            self.run,
            self.settings,
            &self.cancel,
            &self.lease,
        );
        let storage = app.state::<AppStorage>();
        let temp = TempAudioFile::new(processing_temp_audio_path(app, &chunk.job_id, &unit.id)?);
        let path = temp.path().to_path_buf();
        let start = unit.submitted_start_ms;
        let end = unit.submitted_end_ms;
        let run = run.window(start, end);
        let worker_cancel = cancel.clone();
        let worker_lease = lease.clone();
        tauri::async_runtime::spawn_blocking(move || {
            let _lease = worker_lease;
            extract_window(&run, start, end, &path, &worker_cancel)
        })
        .await
        .map_err(|_| {
            ai_error(
                "ai_processing_audio_extract_worker_failed",
                "Speech-window extraction stopped.",
            )
        })??;
        ensure_chunk_active(&storage, chunk)?;
        if settings.ai.transcription.silence_gate_enabled {
            let path = temp.path().to_path_buf();
            let config = whisper_backend::SilenceGateConfig {
                merge_gap_ms: settings.ai.transcription.merge_gap_ms.into(),
                min_speech_ms: settings.ai.transcription.min_speech_ms.into(),
                silence_threshold_db: settings.ai.transcription.silence_threshold_db,
            };
            let loud = tauri::async_runtime::spawn_blocking(move || {
                whisper_backend::detect_speech_ranges(&path, config)
            })
            .await
            .map_err(|_| {
                ai_error(
                    "ai_transcription_vad_worker_failed",
                    "The audio gate stopped.",
                )
            })?
            .map_err(whisper_backend_error)?;
            let owned_start = unit.start_ms - unit.submitted_start_ms;
            let owned_end = unit.end_ms - unit.submitted_start_ms;
            if !loud
                .iter()
                .any(|range| range.start_ms < owned_end && range.end_ms > owned_start)
            {
                return Ok(empty_transcription());
            }
        }
        super::super::super::worker::check_transcription_readiness(app, &storage, settings)?;
        let model = selected_transcription_model(settings)?;
        let language = if staged_language.is_some() {
            staged_language
        } else {
            storage
                .with_connection(|c| {
                    storage::preceding_transcript_language(
                        c,
                        chunk,
                        &track.id,
                        unit.start_ms,
                        &unit.fingerprint,
                        &settings.ai.transcription.provider,
                        &model,
                    )
                })
                .map_err(storage_error)?
        };
        storage
            .with_connection(|c| storage::set_processing_chunk_phase(c, &chunk.id, "transcribing"))
            .map_err(storage_error)?;
        let result = app
            .state::<TranscriptionProviderManager>()
            .transcribe(
                app,
                &storage,
                TranscriptionRequest {
                    processing_lease: Some(lease.clone()),
                    audio_path: temp.path().to_path_buf(),
                    cancel: cancel.clone(),
                    language: language_context::request_language(
                        &settings.ai.transcription.language,
                        language.as_ref(),
                    ),
                    max_initial_timestamp_seconds: settings
                        .ai
                        .transcription
                        .max_initial_timestamp_seconds,
                    model,
                    timeout: TRANSCRIPTION_TIMEOUT,
                    word_timestamps: settings.ai.transcription.word_timestamps_enabled,
                },
                settings,
            )
            .await
            .map_err(transcription_error)?;
        Ok(transcription::constrain_transcription(result, end - start))
    }
}
fn empty_transcription() -> transcription::VerboseTranscription {
    transcription::VerboseTranscription {
        duration: None,
        language: None,
        language_confidence: None,
        segments: Vec::new(),
        words: Vec::new(),
        task: None,
        text: String::new(),
    }
}
