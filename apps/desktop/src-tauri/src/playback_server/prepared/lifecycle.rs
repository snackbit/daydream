//! Prepared files remain owned by sessions and in-flight request snapshots.
use super::*;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, Weak};

#[derive(Default)]
pub(super) struct Owners(HashMap<PathBuf, Weak<FileLease>>);

pub(in crate::playback_server) struct FileLease {
    path: PathBuf,
    storage: AppStorage,
}

pub(super) fn lock() -> Result<MutexGuard<'static, Owners>, String> {
    static OWNERS: OnceLock<Mutex<Owners>> = OnceLock::new();
    OWNERS
        .get_or_init(Mutex::default)
        .lock()
        .map_err(|_| "Prepared file owners unavailable.".into())
}

impl Owners {
    pub(super) fn lease(&mut self, storage: &AppStorage, path: &Path) -> Arc<FileLease> {
        if let Some(owner) = self.0.get(path).and_then(Weak::upgrade) {
            return owner;
        }
        let owner = Arc::new(FileLease {
            path: path.into(),
            storage: storage.clone(),
        });
        self.0.insert(path.into(), Arc::downgrade(&owner));
        owner
    }
}

fn referenced(storage: &AppStorage, path: &Path) -> Result<bool, String> {
    storage
        .with_read_connection(|c| {
            c.query_row(
                "SELECT EXISTS(SELECT 1 FROM studio_media_assets WHERE path=?1)",
                [path.to_string_lossy()],
                |r| r.get(0),
            )
        })
        .map_err(|_| "Prepared file references unavailable.".into())
}

impl Drop for FileLease {
    fn drop(&mut self) {
        let Ok(mut owners) = lock() else {
            return;
        };
        // Another reader may have acquired a new lease while this one was dropping.
        if owners
            .0
            .get(&self.path)
            .is_some_and(|owner| owner.strong_count() > 0)
        {
            return;
        }
        owners.0.remove(&self.path);
        if referenced(&self.storage, &self.path) == Ok(false) {
            // A failed unlink remains discoverable by the next recording sweep.
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

pub(super) fn reclaim(storage: &AppStorage, recording: &RecordingRow) -> Result<(), String> {
    let mut owners = lock()?;
    let directory = Path::new(&recording.storage_dir)
        .join("playback")
        .join(VERSION);
    let entries = match std::fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(_) => return Err("Prepared files could not be listed.".into()),
    };
    for entry in entries {
        let path = entry
            .map_err(|_| "Prepared files could not be listed.")?
            .path();
        if path.extension().is_none_or(|extension| extension != "mp4")
            || owners
                .0
                .get(&path)
                .is_some_and(|owner| owner.strong_count() > 0)
            || referenced(storage, &path)?
        {
            continue;
        }
        std::fs::remove_file(&path)
            .map_err(|_| "Superseded prepared file could not be removed.")?;
        owners.0.remove(&path);
    }
    Ok(())
}
