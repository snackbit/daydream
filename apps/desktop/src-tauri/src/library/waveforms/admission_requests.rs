//! Cache misses request admission without acquiring the processing writer.
use crate::storage::AppStorage;
use std::sync::{Mutex, OnceLock};
fn requests() -> &'static Mutex<Vec<(AppStorage, String)>> {
    static REQUESTS: OnceLock<Mutex<Vec<(AppStorage, String)>>> = OnceLock::new();
    REQUESTS.get_or_init(Mutex::default)
}
pub(super) fn request(storage: &AppStorage, id: &str) -> Result<(), String> {
    let mut pending = requests()
        .lock()
        .map_err(|_| "Waveform preparation could not be queued.")?;
    if !pending
        .iter()
        .any(|(owner, key)| key == id && owner.same_database(storage))
    {
        pending.push((storage.clone(), id.into()));
    }
    Ok(())
}
pub(super) fn flush(storage: &AppStorage) -> Result<(), String> {
    let mut selected = Vec::new();
    requests()
        .lock()
        .map_err(|_| "waveform_admission_failed")?
        .retain(|(owner, id)| {
            if owner.same_database(storage) {
                selected.push(id.clone());
                false
            } else {
                true
            }
        });
    for (index, id) in selected.iter().enumerate() {
        if storage.with_connection(|c|c.execute("INSERT INTO waveform_admission(recording_id) SELECT id FROM recordings WHERE id=?1 ON CONFLICT(recording_id) DO UPDATE SET generation=generation+1",[id])).is_err() {
            for id in &selected[index..] {request(storage,id)?;}
            return Err("waveform_admission_failed".into());
        }
    }
    Ok(())
}
