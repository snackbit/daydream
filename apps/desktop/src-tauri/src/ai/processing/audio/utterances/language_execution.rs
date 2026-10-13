//! Corrections preserve prior publication until eligible replacement succeeds.
use super::*;
use execution::SpeechWorker;
use std::collections::HashMap;

impl SpeechWorker<'_> {
    pub(super) async fn stabilize_language(
        &self,
        unit: &storage::SpeechUnit,
        mut value: transcription::VerboseTranscription,
    ) -> Result<
        (
            transcription::VerboseTranscription,
            HashMap<String, transcription::VerboseTranscription>,
        ),
        AiError,
    > {
        let storage = self.app.state::<AppStorage>();
        let mut existing = storage
            .with_connection(|connection| language_history::decision(connection, &unit.id))
            .map_err(storage_error)?;
        if let Some(decision) = &existing {
            if !storage
                .with_connection(|connection| {
                    language_history::supported(connection, self.chunk, self.track, unit, decision)
                })
                .map_err(storage_error)?
            {
                value = self.transcribe_unit(unit, None).await?;
                publication::checkpoint(&storage, self.chunk, |connection| {
                    connection.execute(
                        "DELETE FROM processing_speech_language_support WHERE unit_id=?1",
                        [&unit.id],
                    )?;
                    connection.execute(
                        "DELETE FROM processing_speech_languages WHERE unit_id=?1",
                        [&unit.id],
                    )?;
                    Ok(true)
                })?;
                existing = None;
            }
        }
        let mut decision = match existing {
            Some(decision) => decision,
            None => {
                let state = storage
                    .with_connection(|connection| {
                        language_history::previous(connection, self.chunk, self.track, unit)
                    })
                    .map_err(storage_error)?;
                language::plan(state, &unit.id, unit.start_ms, unit.end_ms, &value)
            }
        };
        language_history::stage(
            &storage, self.chunk, self.track, unit, &value, &decision, false,
        )?;
        if let Some(target) = decision.correction.clone() {
            value = self.transcribe_unit(unit, Some(&target)).await?;
            decision.output_language = Some(target);
            decision.correction = None;
            language_history::stage(
                &storage, self.chunk, self.track, unit, &value, &decision, true,
            )?;
        }
        let mut staged = HashMap::new();
        if let Some(target) = decision.state.confirmed.clone() {
            for id in decision.repairs.clone() {
                check_atomic_cancel(&self.cancel)?;
                ensure_chunk_active(&storage, self.chunk)?;
                let candidate = storage
                    .with_connection(|connection| {
                        let Some(candidate) = storage::speech_unit(connection, &id)? else {
                            return Ok(None);
                        };
                        if !language_history::eligible(
                            connection, self.chunk, self.track, unit, &candidate,
                        )? || candidate.submitted_start_ms < self.run.start
                            || candidate.submitted_end_ms > self.run.end
                        {
                            return Ok(None);
                        }
                        let Some(prior) = language_history::decision(connection, &id)? else {
                            return Ok(None);
                        };
                        Ok((prior.output_language.as_deref() != Some(target.as_str()))
                            .then_some((candidate, prior)))
                    })
                    .map_err(storage_error)?;
                let Some((candidate, mut prior)) = candidate else {
                    continue;
                };
                let corrected = self.transcribe_unit(&candidate, Some(&target)).await?;
                prior.output_language = Some(target.clone());
                prior.correction = None;
                prior.state = decision.state.clone();
                language_history::replace(
                    &storage,
                    self.chunk,
                    self.track,
                    self.settings,
                    language_history::Replacement {
                        current: unit,
                        unit: &candidate,
                        value: corrected.clone(),
                        language: &prior,
                    },
                )?;
                if candidate.status == "staged" {
                    staged.insert(id, corrected);
                }
            }
        }
        decision.repairs.clear();
        language_history::stage(
            &storage, self.chunk, self.track, unit, &value, &decision, false,
        )?;
        Ok((value, staged))
    }
}
