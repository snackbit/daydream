use super::*;
#[cfg(test)]
mod persistence_tests;

pub(super) fn request_language(
    configured: &str,
    _context: Option<&storage::TranscriptLanguageContext>,
) -> String {
    if configured != "auto" {
        return configured.into();
    }
    "auto".into()
}

pub(super) fn result_context(
    configured: &str,
    previous: Option<storage::TranscriptLanguageContext>,
    transcription: &transcription::VerboseTranscription,
    fingerprint: &str,
    start: i64,
    end: i64,
) -> Option<storage::TranscriptLanguageContext> {
    if configured != "auto"
        || !transcription
            .segments
            .iter()
            .any(|segment| !segment.text.trim().is_empty())
    {
        return None;
    }
    if let Some(context) = previous {
        return transcription
            .language
            .as_ref()
            .is_none_or(|language| language == &context.language)
            .then_some(context);
    }
    let language = transcription.language.as_ref()?;
    let confidence = transcription.language_confidence?;
    if !confidence.is_finite()
        || !(storage::LANGUAGE_CONTEXT_MIN_CONFIDENCE..=1.0).contains(&confidence)
        || !whisper_backend::language_supported(language)
    {
        return None;
    }
    Some(storage::TranscriptLanguageContext {
        language: language.clone(),
        confidence,
        detection_segment_id: None,
        config_fingerprint: fingerprint.into(),
        detection_start_ms: start,
        detection_end_ms: end,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn language_context_requires_confident_speech_and_preserves_explicit_choice() {
        let mut speech =
            crate::ai::tests::test_verbose_transcription(0.0, 1.0, "Synthetic speech", vec![]);
        speech.language = Some("de".into());
        speech.language_confidence = Some(0.9);
        let trusted = result_context("auto", None, &speech, "config", 0, 1000).unwrap();
        assert_eq!(request_language("auto", Some(&trusted)), "auto");
        assert_eq!(request_language("en", Some(&trusted)), "en");
        assert_eq!(request_language("auto", None), "auto");
        for confidence in [0.59, f64::NAN, 1.1] {
            speech.language_confidence = Some(confidence);
            assert!(result_context("auto", None, &speech, "config", 0, 1000).is_none());
        }
        speech.language_confidence = Some(0.6);
        assert!(result_context("auto", None, &speech, "config", 0, 1000).is_some());
        assert!(result_context("de", None, &speech, "config", 0, 1000).is_none());
        speech.language_confidence = None;
        assert!(result_context("auto", None, &speech, "config", 0, 1000).is_none());
        let renewed =
            result_context("auto", Some(trusted.clone()), &speech, "config", 2000, 3000).unwrap();
        assert_eq!(renewed.detection_start_ms, 0);
        speech.language = Some("zh".into());
        assert!(
            result_context("auto", Some(trusted.clone()), &speech, "config", 2000, 3000).is_none()
        );
        speech.segments.clear();
        assert!(result_context("auto", Some(trusted), &speech, "config", 2000, 3000).is_none());
    }
}
