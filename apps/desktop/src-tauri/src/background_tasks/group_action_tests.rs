use super::super::{group_test_fixtures::*, *};
use crate::storage;

#[test]
fn batch_resume_preserves_success_when_another_job_is_still_stopping() {
    let storage = fixture();
    storage
        .with_connection(|connection| {
            recording(connection, "a");
            job(connection, "a", "stopping", 1, "audio", "running");
            job(connection, "a", "ready", 2, "ocr", "queued");
            connection.execute("UPDATE processing_jobs SET status='paused'", [])?;
            Ok(())
        })
        .unwrap();
    let input = ControlRecordingProcessingGroupsInput {
        recording_id: fixture_id("a"),
        action: "resume".into(),
        source_kind: None,
        use_current_settings: false,
    };
    let result = super::super::group_actions::apply(&input, &storage, || false).unwrap();
    assert_eq!(result.results.len(), 2);
    assert!(
        result
            .results
            .iter()
            .find(|result| result.job_id == fixture_id("ready"))
            .unwrap()
            .succeeded
    );
    assert_eq!(
        result
            .results
            .iter()
            .find(|result| result.job_id == fixture_id("stopping"))
            .unwrap()
            .error_code
            .as_deref(),
        Some("ai_processing_resume_pending")
    );
    storage
        .with_connection(|connection| {
            assert_eq!(
                storage::get_processing_job(connection, &fixture_id("ready"))?
                    .unwrap()
                    .status,
                "queued"
            );
            assert_eq!(
                storage::get_processing_job(connection, &fixture_id("stopping"))?
                    .unwrap()
                    .status,
                "paused"
            );
            Ok(())
        })
        .unwrap();
    let cancelled = super::super::group_actions::apply(
        &ControlRecordingProcessingGroupsInput {
            action: "pause".into(),
            ..input
        },
        &storage,
        || true,
    )
    .unwrap();
    assert!(cancelled
        .results
        .iter()
        .all(|result| result.error_code.as_deref() == Some("ai_maintenance_active")));
    storage
        .with_connection(|connection| {
            assert_eq!(
                storage::get_processing_job(connection, &fixture_id("ready"))?
                    .unwrap()
                    .status,
                "queued"
            );
            Ok(())
        })
        .unwrap();
}
