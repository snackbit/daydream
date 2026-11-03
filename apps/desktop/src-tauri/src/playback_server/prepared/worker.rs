use super::*;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, OnceLock,
    },
    time::Duration,
};
use tauri::{Emitter, Manager};

fn owners() -> &'static Mutex<HashMap<String, Arc<AtomicBool>>> {
    static OWNERS: OnceLock<Mutex<HashMap<String, Arc<AtomicBool>>>> = OnceLock::new();
    OWNERS.get_or_init(Mutex::default)
}
pub(crate) fn cancel(id: &str) {
    if let Ok(owners) = owners().lock() {
        if let Some(cancel) = owners.get(id) {
            cancel.store(true, Ordering::Release);
        }
    }
}

pub(crate) fn start(app: tauri::AppHandle) {
    let storage = app.state::<AppStorage>().inner().clone();
    tauri::async_runtime::spawn(async move {
        let recover = storage.clone();
        let _ = tauri::async_runtime::spawn_blocking(move || {
            recover.with_connection(|c| {
                c.execute(
                    "UPDATE studio_media_queue SET status='queued' WHERE status='running'",
                    [],
                )
            })
        })
        .await;
        loop {
            tokio::time::sleep(Duration::from_millis(500)).await;
            let read = storage.clone();
            let next=tauri::async_runtime::spawn_blocking(move ||read.with_read_connection(|c|c.query_row(
                "SELECT q.recording_id,q.generation FROM studio_media_queue q JOIN recordings r ON r.id=q.recording_id WHERE q.status='queued' AND q.priority>0 AND r.status!='planned' AND NOT EXISTS(SELECT 1 FROM recording_protection_spans s WHERE s.recording_id=q.recording_id AND s.status!='materialized') ORDER BY q.priority DESC,q.updated_at_ms LIMIT 1",[],|r|Ok((r.get::<_,String>(0)?,r.get::<_,i64>(1)?))
            ).optional())).await;
            let Ok(Ok(Some((id, generation)))) = next else {
                continue;
            };
            let Ok(Some(activity)) = crate::maintenance::runtime::admit(&app, "media-preparation")
            else {
                continue;
            };
            let cancellation = activity.cancellation();
            let cancelled = Arc::new(AtomicBool::new(false));
            if let Ok(mut owners) = owners().lock() {
                owners.insert(id.clone(), cancelled.clone());
            }
            let lease = crate::recording_privacy::processing_lease(&id).await;
            let work_storage = storage.clone();
            let work_id = id.clone();
            let cancel_work = cancelled.clone();
            let operation = crate::background_tasks::operations::Operation::start(
                &app,
                "media-preparation",
                &id,
                "Playback and timeline preparation",
            );
            let mut work = tauri::async_runtime::spawn_blocking(move || {
                let _lease = lease;
                let _activity = activity;
                let claimed = work_storage.with_connection(|c|c.execute("UPDATE studio_media_queue SET status='running' WHERE recording_id=?1 AND generation=?2 AND status='queued'",params![work_id,generation])).map_err(|_|"Preparation admission failed.".to_owned())?;
                if claimed == 0 {
                    return Err("Preparation superseded.".to_owned());
                }
                let result = super::generation::recording(&work_storage, &work_id, &cancel_work);
                let interrupted = cancel_work.load(Ordering::Acquire);
                work_storage.with_connection(|c|c.execute(
                    "UPDATE studio_media_queue SET status=?3,error_code=?4,updated_at_ms=?5 WHERE recording_id=?1 AND generation=?2",
                    params![work_id,generation,if interrupted {"queued"}else if result.is_ok(){"ready"}else{"failed"},result.as_ref().err().map(|_|"studio_media_preparation_failed"),crate::background_tasks::now_ms()],
                ).map(|_|())).map_err(|_|"Preparation completion failed.".to_owned())?;
                result
            });
            let result = tokio::select! {
                result=&mut work=>result,
                _=cancellation.cancelled()=>{
                    cancelled.store(true,Ordering::Release);
                    work.await
                }
            };
            if let Ok(mut owners) = owners().lock() {
                owners.remove(&id);
            }
            let success = matches!(result, Ok(Ok(())));
            operation.finish((!success).then_some("ai_media_preparation_failed"));
            if success {
                let _ = app.emit(
                    "library-media-prepared",
                    serde_json::json!({"recordingId":id}),
                );
            }
        }
    });
}
