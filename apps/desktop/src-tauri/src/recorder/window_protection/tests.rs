use super::{
    matching::rule_matches_window,
    state::{ProtectionDecision, ProtectionState},
};
use crate::{
    recorder::types::{ProtectableWindow, ProtectableWindowProviderStatus, WindowMetadataSnapshot},
    settings::{
        ProtectedWindowAction, ProtectedWindowRule, ProtectedWindowRuleAction,
        RecordWindowProtectionSettings,
    },
    storage::{self, AppStorage, CreateRecordingInput},
};
const RECORDING_ID: &str = "00000000-0000-7000-8000-000000000001";

fn settings() -> RecordWindowProtectionSettings {
    RecordWindowProtectionSettings {
        default_action: ProtectedWindowAction::Pause,
        dwell_ms: 2_000,
        enabled: true,
        merge_gap_ms: 5_000,
        rules: vec![rule("private-a"), rule("private-b")],
    }
}
fn rule(id: &str) -> ProtectedWindowRule {
    ProtectedWindowRule {
        action: ProtectedWindowRuleAction::Inherit,
        app_identity: None,
        enabled: true,
        id: id.into(),
        label: id.into(),
        process_name: Some(id.into()),
        title_matcher: None,
    }
}
fn snapshot(process: Option<&str>) -> WindowMetadataSnapshot {
    WindowMetadataSnapshot {
        focused_window: process.map(|name| ProtectableWindow {
            app_identity: Some(name.into()),
            id: "window".into(),
            platform: "test".into(),
            process_id: None,
            process_name: Some(name.into()),
            title: Some("Private chat".into()),
        }),
        provider: Some("test".into()),
        status: ProtectableWindowProviderStatus::Available,
        windows: vec![],
    }
}
fn storage() -> AppStorage {
    let storage =
        storage::test_storage(std::env::temp_dir().join("daydream-protection-tests.sqlite"));
    storage
        .with_connection(|connection| {
            storage::create_recording(
                connection,
                &CreateRecordingInput {
                    id: RECORDING_ID.into(),
                    active_scene_id: None,
                    created_at_ms: 0,
                    default_fps: 1.,
                    notes: None,
                    output_root: None,
                    quality_label: "balanced-ocr".into(),
                    storage_dir: "/tmp/daydream-protection-tests".into(),
                },
            )
        })
        .expect("recording");
    storage
}
fn tick(
    state: &mut ProtectionState,
    storage: &AppStorage,
    time: u64,
    process: Option<&str>,
) -> Result<ProtectionDecision, crate::recorder::types::RecorderError> {
    state.observe(
        storage,
        RECORDING_ID,
        0,
        &settings(),
        time,
        snapshot(process),
    )
}

#[test]
fn continuous_protected_rule_changes_never_resume_recording() {
    let storage = storage();
    let mut state = ProtectionState::default();
    assert_eq!(
        tick(&mut state, &storage, 1_000, Some("private-a")).unwrap(),
        ProtectionDecision::None
    );
    assert_eq!(
        tick(&mut state, &storage, 3_000, Some("private-a")).unwrap(),
        ProtectionDecision::Pause
    );
    for timestamp in [4_000, 9_000, 15_000] {
        assert_eq!(
            tick(&mut state, &storage, timestamp, Some("private-b")).unwrap(),
            ProtectionDecision::None
        );
    }
    assert_eq!(
        tick(&mut state, &storage, 19_999, None).unwrap(),
        ProtectionDecision::None
    );
    assert_eq!(
        tick(&mut state, &storage, 20_000, None).unwrap(),
        ProtectionDecision::Resume
    );
    let redactions = storage
        .with_connection(|connection| storage::list_redactions(connection, RECORDING_ID))
        .unwrap();
    assert_eq!(redactions.len(), 1);
    assert_eq!(redactions[0].start_offset_ms, 1_000);
    assert_eq!(redactions[0].end_offset_ms, 20_000);
    assert_eq!(
        tick(&mut state, &storage, 20_250, Some("private-a")).unwrap(),
        ProtectionDecision::None,
        "new visit must observe dwell again"
    );
}

#[test]
fn provider_failure_retains_open_protection_and_reports_recording_failure() {
    let storage = storage();
    let mut state = ProtectionState::default();
    tick(&mut state, &storage, 1_000, Some("private-a")).unwrap();
    tick(&mut state, &storage, 3_000, Some("private-a")).unwrap();
    let mut unavailable = snapshot(None);
    unavailable.status = ProtectableWindowProviderStatus::ProviderUnavailable;
    let error = state
        .observe(&storage, RECORDING_ID, 0, &settings(), 10_000, unavailable)
        .unwrap_err();
    assert_eq!(error.code, "recorder_window_protection_lost");
    assert_eq!(
        storage
            .with_connection(|connection| storage::list_open_recording_protection_spans(
                connection,
                RECORDING_ID
            ))
            .unwrap()
            .len(),
        1
    );
    assert!(storage
        .with_connection(|connection| storage::list_redactions(connection, RECORDING_ID))
        .unwrap()
        .is_empty());
}

#[test]
fn failed_span_creation_reports_failure_instead_of_continuing_capture() {
    let storage = storage();
    let mut state = ProtectionState::default();
    storage.with_connection(|connection| connection.execute_batch("CREATE TRIGGER deny_open BEFORE INSERT ON recording_protection_spans BEGIN SELECT RAISE(FAIL, 'simulated storage failure'); END;")).unwrap();
    tick(&mut state, &storage, 1_000, Some("private-a")).unwrap();
    assert_eq!(
        tick(&mut state, &storage, 3_000, Some("private-a"))
            .unwrap_err()
            .code,
        "recorder_window_protection_storage_failed"
    );
}

#[test]
fn failed_span_close_retains_protection_and_does_not_allow_resume() {
    let storage = storage();
    let mut state = ProtectionState::default();
    tick(&mut state, &storage, 1_000, Some("private-a")).unwrap();
    tick(&mut state, &storage, 3_000, Some("private-a")).unwrap();
    storage.with_connection(|connection| connection.execute_batch("CREATE TRIGGER deny_close BEFORE UPDATE ON recording_protection_spans BEGIN SELECT RAISE(FAIL, 'simulated storage failure'); END;")).unwrap();
    assert_eq!(
        tick(&mut state, &storage, 8_000, None).unwrap_err().code,
        "recorder_window_protection_storage_failed"
    );
    assert_eq!(
        storage
            .with_connection(|connection| storage::list_open_recording_protection_spans(
                connection,
                RECORDING_ID
            ))
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn redaction_materialization_failure_rolls_back_close_and_can_retry() {
    let storage = storage();
    let mut state = ProtectionState::default();
    tick(&mut state, &storage, 1_000, Some("private-a")).unwrap();
    tick(&mut state, &storage, 3_000, Some("private-a")).unwrap();
    storage.with_connection(|connection| connection.execute_batch("CREATE TRIGGER deny_redaction BEFORE INSERT ON redactions BEGIN SELECT RAISE(FAIL, 'simulated storage failure'); END;")).unwrap();
    assert!(tick(&mut state, &storage, 8_000, None).is_err());
    assert_eq!(
        storage
            .with_connection(|connection| storage::list_open_recording_protection_spans(
                connection,
                RECORDING_ID
            ))
            .unwrap()
            .len(),
        1
    );
    storage
        .with_connection(|connection| connection.execute_batch("DROP TRIGGER deny_redaction;"))
        .unwrap();
    assert_eq!(
        tick(&mut state, &storage, 8_000, None).unwrap(),
        ProtectionDecision::Resume
    );
    assert_eq!(
        storage
            .with_connection(|connection| storage::list_redactions(connection, RECORDING_ID))
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn rule_matches_process_or_title() {
    let mut rule = rule("game");
    rule.title_matcher = Some(crate::settings::ProtectedWindowTitleMatcher {
        case_sensitive: false,
        kind: crate::settings::ProtectedWindowTitleMatcherKind::Contains,
        value: "private".into(),
    });
    assert!(rule_matches_window(
        &rule,
        &snapshot(Some("game")).focused_window.unwrap()
    ));
    assert!(rule_matches_window(
        &rule,
        &snapshot(Some("other")).focused_window.unwrap()
    ));
    let mut window = snapshot(Some("other")).focused_window.unwrap();
    window.title = Some("Notes".into());
    assert!(!rule_matches_window(&rule, &window));
}
