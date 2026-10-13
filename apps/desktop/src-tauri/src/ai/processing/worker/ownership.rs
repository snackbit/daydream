//! A slot remains owned until the job and all spawned source workers have stopped.
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Clone, Default)]
pub(crate) struct ProcessingWorkerOwners(Arc<Mutex<HashMap<String, String>>>);

pub(super) struct WorkerOwnership {
    registry: ProcessingWorkerOwners,
    id: String,
}
impl ProcessingWorkerOwners {
    pub(crate) fn snapshot(&self) -> HashMap<String, String> {
        self.0
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
    }
    pub(super) fn reserve(&self, id: &str, kind: &str) -> Option<Arc<WorkerOwnership>> {
        let mut owners = self.0.lock().unwrap_or_else(|error| error.into_inner());
        if owners.contains_key(id) {
            return None;
        }
        owners.insert(id.into(), kind.into());
        Some(Arc::new(WorkerOwnership {
            registry: self.clone(),
            id: id.into(),
        }))
    }
}
impl Drop for WorkerOwnership {
    fn drop(&mut self) {
        self.registry
            .0
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .remove(&self.id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_workers_keep_the_slot_until_the_last_owner_stops() {
        let owners = ProcessingWorkerOwners::default();
        let job = owners.reserve("job", "ai-processing").unwrap();
        let source = job.clone();
        drop(job);
        assert!(owners.reserve("job", "ai-processing").is_none());
        assert_eq!(owners.snapshot().len(), 1);
        drop(source);
        assert!(owners.snapshot().is_empty());
        assert!(owners.reserve("job", "ai-processing").is_some());
    }
    #[test]
    fn worker_unwind_does_not_release_a_still_running_source_or_poison_admission() {
        let owners = ProcessingWorkerOwners::default();
        let job = owners.reserve("job", "ai-processing").unwrap();
        let source = job.clone();
        assert!(std::panic::catch_unwind(move || {
            let _job = job;
            panic!("synthetic worker failure");
        })
        .is_err());
        assert!(owners.reserve("job", "ai-processing").is_none());
        let cloned = owners.clone();
        let _ = std::thread::spawn(move || {
            let _lock = cloned.0.lock().unwrap();
            panic!("synthetic poisoned registry");
        })
        .join();
        assert_eq!(owners.snapshot().len(), 1);
        drop(source);
        assert!(owners.snapshot().is_empty());
        assert!(owners.reserve("job", "ai-processing").is_some());
    }
}
