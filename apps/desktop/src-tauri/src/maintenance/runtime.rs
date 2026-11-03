//! Admission and completion ownership for destructive maintenance.
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};
use tauri::Manager;
use tokio::sync::Notify;

pub(crate) fn admit<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    kind: &str,
) -> Result<Option<RuntimeActivityLease>, String> {
    match app.try_state::<RuntimeActivityRegistry>() {
        Some(registry) => registry.begin(kind).map(Some),
        #[cfg(test)]
        None => Ok(None),
        #[cfg(not(test))]
        None => Err("Runtime activity control is unavailable. Restart Daydream.".into()),
    }
}

#[derive(Clone, Default)]
pub(crate) struct RuntimeActivityRegistry {
    inner: Arc<Inner>,
}
#[derive(Default)]
struct Inner {
    state: Mutex<State>,
    changed: Notify,
}
#[derive(Default)]
struct State {
    exclusive: bool,
    next: u64,
    activities: HashMap<u64, RuntimeCancellation>,
}
#[derive(Clone, Default)]
pub(crate) struct RuntimeCancellation {
    cancelled: Arc<AtomicBool>,
    changed: Arc<Notify>,
}
impl RuntimeCancellation {
    pub(crate) fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }
    pub(crate) fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
        self.changed.notify_waiters();
    }
    pub(crate) async fn cancelled(&self) {
        loop {
            let notified = self.changed.notified();
            if self.is_cancelled() {
                return;
            }
            notified.await;
        }
    }
}
pub(crate) struct RuntimeActivityLease {
    registry: RuntimeActivityRegistry,
    id: u64,
    cancellation: RuntimeCancellation,
}
pub(crate) struct MaintenanceLease {
    registry: RuntimeActivityRegistry,
    release: bool,
}
impl RuntimeActivityRegistry {
    pub(crate) fn begin(&self, _kind: &str) -> Result<RuntimeActivityLease, String> {
        let mut state = self
            .inner
            .state
            .lock()
            .map_err(|_| "Runtime activity state is unavailable.")?;
        if state.exclusive {
            return Err(
                "Maintenance is in progress. Finish or retry it before starting new work.".into(),
            );
        }
        state.next += 1;
        let id = state.next;
        let cancellation = RuntimeCancellation::default();
        state.activities.insert(id, cancellation.clone());
        Ok(RuntimeActivityLease {
            registry: self.clone(),
            id,
            cancellation,
        })
    }
    pub(crate) fn begin_maintenance(&self) -> Result<MaintenanceLease, String> {
        let mut state = self
            .inner
            .state
            .lock()
            .map_err(|_| "Runtime activity state is unavailable.")?;
        if state.exclusive {
            return Err("Another maintenance operation is already pending.".into());
        }
        state.exclusive = true;
        Ok(MaintenanceLease {
            registry: self.clone(),
            release: true,
        })
    }
    pub(crate) fn is_maintenance_active(&self) -> bool {
        self.inner
            .state
            .lock()
            .map(|state| state.exclusive)
            .unwrap_or(true)
    }
    pub(crate) fn resume_maintenance(&self) -> Result<MaintenanceLease, String> {
        let mut state = self
            .inner
            .state
            .lock()
            .map_err(|_| "Runtime activity state is unavailable.")?;
        state.exclusive = true;
        Ok(MaintenanceLease {
            registry: self.clone(),
            release: true,
        })
    }
    pub(crate) fn block_for_recovery(&self) {
        if let Ok(mut state) = self.inner.state.lock() {
            state.exclusive = true;
        }
    }
}
impl RuntimeActivityLease {
    pub(crate) fn cancellation(&self) -> RuntimeCancellation {
        self.cancellation.clone()
    }
}
impl Drop for RuntimeActivityLease {
    fn drop(&mut self) {
        if let Ok(mut state) = self.registry.inner.state.lock() {
            state.activities.remove(&self.id);
        }
        self.registry.inner.changed.notify_waiters();
    }
}
impl MaintenanceLease {
    /// Once a durable marker exists, every early-return path preserves the gate.
    pub(crate) fn protect_recovery(&mut self) {
        self.release = false;
    }
    /// Only a fully completed cleanup may reopen admission.
    pub(crate) fn release(mut self) {
        self.release = true;
    }
    pub(crate) async fn cancel_and_drain(&self) -> Result<(), String> {
        {
            let state = self
                .registry
                .inner
                .state
                .lock()
                .map_err(|_| "Runtime activity state is unavailable.")?;
            for cancellation in state.activities.values() {
                cancellation.cancel();
            }
        }
        tokio::time::timeout(std::time::Duration::from_secs(30), async {
            loop {
                let notified = self.registry.inner.changed.notified();
                if self
                    .registry
                    .inner
                    .state
                    .lock()
                    .map(|state| state.activities.is_empty())
                    .unwrap_or(false)
                {
                    break;
                }
                notified.await;
            }
        })
        .await
        .map_err(|_| {
            "Background work did not stop. Restart Daydream before retrying maintenance."
                .to_string()
        })
    }
    /// Persisted partial cleanup must keep admission closed until retry/restart.
    pub(crate) fn keep_closed(mut self) {
        self.release = false;
    }
}
impl Drop for MaintenanceLease {
    fn drop(&mut self) {
        if self.release {
            if let Ok(mut state) = self.registry.inner.state.lock() {
                state.exclusive = false;
            }
            self.registry.inner.changed.notify_waiters();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn exclusive_cancels_work_and_waits_for_owner_completion() {
        let registry = RuntimeActivityRegistry::default();
        let activity = registry.begin("synthetic").unwrap();
        let cancellation = activity.cancellation();
        let maintenance = registry.begin_maintenance().unwrap();
        assert!(registry.begin("later").is_err());
        {
            let drain = maintenance.cancel_and_drain();
            tokio::pin!(drain);
            tokio::select! { _ = &mut drain => panic!("owner still running"), _ = cancellation.cancelled() => {} }
            drop(activity);
            drain.await.unwrap();
        }
        drop(maintenance);
        assert!(registry.begin("after").is_ok());
    }
}
