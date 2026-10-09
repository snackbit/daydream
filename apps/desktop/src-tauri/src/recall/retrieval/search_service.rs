//! Typed command boundary for calendar-scoped recall reads.
use super::{
    search::{self, SearchScope},
    types::{RecallSearchHit, RecallSearchInput, RecallSearchResult},
};
use crate::storage::AppStorage;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use tauri::Manager;

#[derive(Default)]
pub struct SearchControl {
    current: Mutex<Option<Arc<AtomicBool>>>,
}
struct SearchLease<'a> {
    control: &'a SearchControl,
    cancelled: Arc<AtomicBool>,
}
impl SearchControl {
    fn begin(&self) -> Result<SearchLease<'_>, String> {
        let mut current = self
            .current
            .lock()
            .map_err(|_| "Recall search control unavailable")?;
        if let Some(previous) = current.take() {
            previous.store(true, Ordering::Release);
        }
        let cancelled = Arc::new(AtomicBool::new(false));
        *current = Some(cancelled.clone());
        Ok(SearchLease {
            control: self,
            cancelled,
        })
    }
    pub fn cancel(&self) -> Result<(), String> {
        let current = self
            .current
            .lock()
            .map_err(|_| "Recall search control unavailable")?;
        if let Some(current) = current.as_ref() {
            current.store(true, Ordering::Release);
        }
        Ok(())
    }
}
impl Drop for SearchLease<'_> {
    fn drop(&mut self) {
        if let Ok(mut current) = self.control.current.lock() {
            if current
                .as_ref()
                .is_some_and(|c| Arc::ptr_eq(c, &self.cancelled))
            {
                *current = None;
            }
        }
    }
}
pub fn cancel_search(app: &tauri::AppHandle) -> Result<(), String> {
    app.state::<SearchControl>().cancel()
}

pub async fn search_evidence(
    app: &tauri::AppHandle,
    input: &RecallSearchInput,
) -> Result<RecallSearchResult, String> {
    let _activity = crate::maintenance::runtime::admit(app, "recall-search")?;
    let control = app.state::<SearchControl>();
    let lease = control.begin()?;
    if input
        .mode
        .as_deref()
        .is_some_and(|m| m != "hybrid" && m != "lexical")
    {
        return Err("Recall search mode is invalid".into());
    }
    let mut scope = SearchScope {
        start_ms: input.from_ms,
        end_ms: input.to_ms,
        source_kinds: input.source_kinds.clone(),
        cursor: input.cursor.clone(),
        lexical_only: input.mode.as_deref() == Some("lexical"),
        ..Default::default()
    };
    if let Some(day) = &input.day_id {
        let timezone = input
            .timezone
            .as_deref()
            .ok_or("A timezone is required for calendar-day search")?;
        let (start, end) = super::day::day_bounds(day, timezone)?;
        scope.start_ms = Some(scope.start_ms.map_or(start, |value| value.max(start)));
        scope.end_ms = Some(scope.end_ms.map_or(end, |value| value.min(end)));
    }
    let provider = if scope.lexical_only {
        None
    } else {
        super::provider_config::build_embedding_provider(app).ok()
    };
    let storage = app.state::<AppStorage>();
    let page = search::search(
        &storage,
        provider.as_ref(),
        &input.query,
        &scope,
        input.limit,
        &lease.cancelled,
    )
    .await?;
    Ok(RecallSearchResult {
        privacy_revision: page.privacy_revision,
        hits: page
            .hits
            .into_iter()
            .map(|(evidence, score)| RecallSearchHit { evidence, score })
            .collect(),
        complete: page.complete,
        unavailable_reason: page.unavailable_reason,
        next_cursor: page.next_cursor,
        semantic_used: page.semantic_used,
        indexed_evidence_count: page.indexed_evidence_count,
        eligible_evidence_count: page.eligible_evidence_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn newer_query_cancels_previous_and_old_completion_cannot_clear_it() {
        let control = SearchControl::default();
        let first = control.begin().unwrap();
        let second = control.begin().unwrap();
        assert!(first.cancelled.load(Ordering::Acquire));
        assert!(!second.cancelled.load(Ordering::Acquire));
        drop(first);
        control.cancel().unwrap();
        assert!(second.cancelled.load(Ordering::Acquire));
        drop(second);
        assert!(control.current.lock().unwrap().is_none());
    }
}
