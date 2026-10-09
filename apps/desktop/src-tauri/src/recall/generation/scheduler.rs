//! Generation timing policy shared by the durable worker and its tests.
pub const QUIET_MS: i64 = 5 * 60 * 1000;
pub const RECORDING_INTERVAL_MS: i64 = 15 * 60 * 1000;

pub fn generation_due(
    now_ms: i64,
    changed_at_ms: i64,
    last_started_ms: Option<i64>,
    recording: bool,
    manual: bool,
) -> bool {
    manual
        || (now_ms.saturating_sub(changed_at_ms) >= QUIET_MS
            && (!recording
                || last_started_ms
                    .is_none_or(|last| now_ms.saturating_sub(last) >= RECORDING_INTERVAL_MS)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn automatic_generation_waits_for_quiet_period_and_recording_quota() {
        assert!(!generation_due(QUIET_MS - 1, 0, None, false, false));
        assert!(generation_due(QUIET_MS, 0, None, false, false));
        assert!(!generation_due(
            RECORDING_INTERVAL_MS - 1,
            0,
            Some(0),
            true,
            false
        ));
        assert!(generation_due(
            RECORDING_INTERVAL_MS,
            0,
            Some(0),
            true,
            false
        ));
        assert!(generation_due(QUIET_MS, 0, Some(QUIET_MS), false, false));
        assert!(!generation_due(0, 1, None, false, false));
    }

    #[test]
    fn explicit_regeneration_bypasses_automatic_cadence() {
        assert!(generation_due(0, 0, None, false, true));
        assert!(generation_due(1, 1, Some(0), true, true));
    }
}
