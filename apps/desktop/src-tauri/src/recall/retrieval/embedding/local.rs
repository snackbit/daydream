use super::EmbeddingPurpose;
use crate::settings::RecallEmbeddingSettings;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, OnceLock,
    },
    time::{Duration, Instant},
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{Child, ChildStdin, ChildStdout, Command},
    sync::Mutex,
};

pub(super) struct LocalWorker {
    models_dir: PathBuf,
    worker_path: PathBuf,
    state: Arc<Mutex<Option<Running>>>,
}
struct Running {
    child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
    identity: String,
    last_used: Instant,
}
static SHARED: OnceLock<Arc<Mutex<Option<Running>>>> = OnceLock::new();
static RETIRED: std::sync::Mutex<Vec<Child>> = std::sync::Mutex::new(Vec::new());

fn retire_running(state: &mut Option<Running>) {
    if let Some(Running { mut child, .. }) = state.take() {
        let _ = child.start_kill();
        RETIRED
            .lock()
            .expect("embedding retired children")
            .push(child);
    }
}

/// Admission and active inference must drain before this cleanup begins.
pub(super) async fn shutdown_for_maintenance() -> Result<(), String> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
    if let Some(shared) = SHARED.get() {
        let mut state = tokio::time::timeout_at(deadline, shared.lock())
            .await
            .map_err(|_| {
                "The local embedding worker is still stopping. Retry maintenance.".to_string()
            })?;
        if let Some(running) = state.as_mut() {
            running
                .child
                .start_kill()
                .map_err(|_| "Could not stop the local embedding worker.".to_string())?;
            tokio::time::timeout_at(deadline, running.child.wait())
                .await
                .map_err(|_| {
                    "The local embedding worker did not stop. Retry maintenance.".to_string()
                })?
                .map_err(|_| "Could not await the local embedding worker.".to_string())?;
        }
        state.take();
    }
    drain_retired(deadline).await
}

async fn drain_retired(deadline: tokio::time::Instant) -> Result<(), String> {
    let retired = {
        std::mem::take(
            &mut *RETIRED
                .lock()
                .map_err(|_| "Could not drain embedding workers.".to_string())?,
        )
    };
    let mut remaining = retired.into_iter();
    while let Some(mut child) = remaining.next() {
        if !matches!(
            tokio::time::timeout_at(deadline, child.wait()).await,
            Ok(Ok(_))
        ) {
            let mut retired = RETIRED
                .lock()
                .map_err(|_| "Could not retain stopped embedding workers.".to_string())?;
            retired.push(child);
            retired.extend(remaining);
            return Err("Could not await a stopped embedding worker.".into());
        }
    }
    Ok(())
}
#[derive(Deserialize)]
struct Reply {
    vectors: Option<Vec<Vec<f32>>>,
    error: Option<String>,
    ready: Option<bool>,
}

struct RequestLease<'a> {
    state: &'a mut Option<Running>,
    completed: bool,
}

impl Drop for RequestLease<'_> {
    fn drop(&mut self) {
        if !self.completed {
            // A dropped future may leave an unread reply in the pipe. Child's
            // kill_on_drop invalidates the transport before another caller can
            // acquire the state; that reply can never become another query's.
            retire_running(self.state);
        }
    }
}

impl LocalWorker {
    pub(super) fn new(models_dir: PathBuf, worker_path: PathBuf) -> Self {
        // Providers are snapshots per request; keep one bounded process cache
        // across snapshots so repeated searches reuse the loaded model.
        Self {
            models_dir,
            worker_path,
            state: SHARED.get_or_init(|| Arc::new(Mutex::new(None))).clone(),
        }
    }
    pub(super) async fn embed(
        &self,
        settings: &RecallEmbeddingSettings,
        purpose: EmbeddingPurpose,
        texts: &[String],
        cancelled: &AtomicBool,
    ) -> Result<Vec<Vec<f32>>, String> {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(120);
        // Search, indexing and settings probes share one native inference slot.
        static CAPACITY: OnceLock<tokio::sync::Semaphore> = OnceLock::new();
        let capacity = CAPACITY.get_or_init(|| tokio::sync::Semaphore::new(1));
        let _permit = loop {
            tokio::select! {
                permit = capacity.acquire() => break permit.map_err(|_| "recall_embedding_worker_unavailable")?,
                _ = tokio::time::sleep_until(deadline) => return Err("recall_embedding_timeout".into()),
                _ = tokio::time::sleep(Duration::from_millis(25)) => if cancelled.load(Ordering::Acquire) { return Err("recall_cancelled".into()); }
            }
        };
        let identity = format!(
            "{}:{}:{}:{}",
            settings.local_model_id,
            settings.local_backend,
            self.models_dir.display(),
            self.worker_path.display()
        );
        let mut state = loop {
            tokio::select! {
                state=self.state.lock()=>break state,
                _ = tokio::time::sleep_until(deadline) => return Err("recall_embedding_timeout".into()),
                _=tokio::time::sleep(Duration::from_millis(25))=>if cancelled.load(Ordering::Acquire) { return Err("recall_cancelled".into()); }
            }
        };
        if state
            .as_ref()
            .is_some_and(|running| running.identity != identity)
        {
            retire_running(&mut state);
        }
        drain_retired(tokio::time::Instant::now() + Duration::from_secs(3)).await?;
        let mut lease = RequestLease {
            state: &mut state,
            completed: false,
        };
        let operation = async {
            if lease.state.is_none() {
                let path = self
                    .models_dir
                    .join("embeddings")
                    .join(format!("{}.gguf", settings.local_model_id));
                verify_model(&path, &settings.local_model_id).await?;
                *lease.state = Some(
                    start_configured_worker(&self.worker_path, &path, settings, &identity).await?,
                );
            }
            let running = lease
                .state
                .as_mut()
                .ok_or("recall_embedding_worker_failed")?;
            let mut request=serde_json::to_vec(&serde_json::json!({"texts":texts,"purpose":match purpose {EmbeddingPurpose::Query=>"query",EmbeddingPurpose::Document=>"document"}})).map_err(|_|"recall_embedding_request_invalid")?;
            request.push(b'\n');
            if request.len() > 300000 {
                return Err("recall_embedding_input_limit".into());
            }
            running
                .input
                .write_all(&request)
                .await
                .map_err(|_| "recall_embedding_worker_crashed")?;
            running
                .input
                .flush()
                .await
                .map_err(|_| "recall_embedding_worker_crashed")?;
            let reply: Reply =
                serde_json::from_slice(&read_bounded_line(&mut running.output).await?)
                    .map_err(|_| "recall_embedding_worker_protocol")?;
            running.last_used = Instant::now();
            reply.vectors.ok_or_else(|| {
                match reply.error.as_deref() {
                    Some("text_token_limit") => "recall_embedding_token_limit",
                    _ => "recall_embedding_inference_failed",
                }
                .to_string()
            })
        };
        let result = {
            tokio::pin!(operation);
            let deadline = tokio::time::sleep_until(deadline);
            tokio::pin!(deadline);
            loop {
                tokio::select! {
                    result=&mut operation=>break result,
                    _=&mut deadline=>break Err("recall_embedding_timeout".into()),
                    _=tokio::time::sleep(Duration::from_millis(25))=>if cancelled.load(Ordering::Acquire) {break Err("recall_cancelled".into());}
                }
            }
        };
        if result.is_err() {
            if let Some(mut running) = lease.state.take() {
                let _ = running.child.kill().await;
            }
        }
        lease.completed = true;
        drop(lease);
        drop(state);
        // One timer per batch; only the timer observing an idle worker may unload it.
        let weak = Arc::downgrade(&self.state);
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(60)).await;
            if let Some(state) = weak.upgrade() {
                let mut state = state.lock().await;
                if state
                    .as_ref()
                    .is_some_and(|worker| worker.last_used.elapsed() >= Duration::from_secs(60))
                {
                    if let Some(mut worker) = state.take() {
                        let _ = worker.child.kill().await;
                    }
                }
            }
        });
        result
    }
}

async fn start_configured_worker(
    cpu: &Path,
    model: &Path,
    settings: &RecallEmbeddingSettings,
    identity: &str,
) -> Result<Running, String> {
    let gpu = cpu.with_file_name(if cfg!(windows) {
        "daydream-embedding-worker-vulkan.exe"
    } else {
        "daydream-embedding-worker-vulkan"
    });
    if settings.local_backend == "vulkan" {
        if !gpu.is_file() {
            return Err("recall_embedding_vulkan_unavailable".into());
        }
        return start_worker(&gpu, model, &settings.local_model_id, "vulkan", identity).await;
    }
    if settings.local_backend == "auto" && gpu.is_file() {
        if let Ok(Ok(worker)) = tokio::time::timeout(
            Duration::from_secs(30),
            start_worker(&gpu, model, &settings.local_model_id, "auto", identity),
        )
        .await
        {
            return Ok(worker);
        }
    }
    start_worker(cpu, model, &settings.local_model_id, "cpu", identity).await
}

async fn start_worker(
    executable: &Path,
    model: &Path,
    id: &str,
    backend: &str,
    identity: &str,
) -> Result<Running, String> {
    let mut child = Command::new(executable)
        .arg(model)
        .arg(id)
        .arg(backend)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|_| "recall_embedding_worker_unavailable")?;
    let input = child.stdin.take().ok_or("recall_embedding_worker_failed")?;
    let mut output = BufReader::new(
        child
            .stdout
            .take()
            .ok_or("recall_embedding_worker_failed")?,
    );
    let ready: Reply = serde_json::from_slice(&read_bounded_line(&mut output).await?)
        .map_err(|_| "recall_embedding_worker_protocol")?;
    if ready.ready != Some(true) {
        return Err(match ready.error.as_deref() {
            Some("vulkan_unavailable") => "recall_embedding_vulkan_unavailable",
            _ => "recall_embedding_model_incompatible",
        }
        .to_string());
    }
    Ok(Running {
        child,
        input,
        output,
        identity: identity.into(),
        last_used: Instant::now(),
    })
}

async fn read_bounded_line(reader: &mut BufReader<ChildStdout>) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    loop {
        let available = reader
            .fill_buf()
            .await
            .map_err(|_| "recall_embedding_worker_crashed")?;
        if available.is_empty() {
            return Err("recall_embedding_worker_crashed".into());
        }
        let newline = available.iter().position(|byte| *byte == b'\n');
        let used = newline.map_or(available.len(), |offset| offset + 1);
        if bytes.len() + used > 1024 * 1024 {
            return Err("recall_embedding_worker_response_limit".into());
        }
        bytes.extend_from_slice(&available[..used]);
        reader.consume(used);
        if newline.is_some() {
            return Ok(bytes);
        }
    }
}

async fn verify_model(path: &Path, id: &str) -> Result<(), String> {
    let expected = match id {
        "multilingual-e5-small-q8_0" => {
            "0a34067a40f25d3149b36885faa62bee0e5284d0f9edc102acfc00e115d953e8"
        }
        "bge-small-en-v1.5-q8_0" => {
            "f046db1dc724cf4f6f0a0c5917e922823b73eb1d27b8f9a9c2797f7866974804"
        }
        _ => return Err("recall_embedding_model_unknown".into()),
    };
    let path = path.to_owned();
    let hash = tokio::task::spawn_blocking(move || {
        use std::io::Read;
        let mut file = std::fs::File::open(path).map_err(|_| "recall_embedding_model_missing")?;
        let mut hash = Sha256::new();
        let mut buffer = [0u8; 65536];
        loop {
            let count = file
                .read(&mut buffer)
                .map_err(|_| "recall_embedding_model_read_failed")?;
            if count == 0 {
                break;
            }
            hash.update(&buffer[..count]);
        }
        Ok::<_, String>(hex::encode(hash.finalize()))
    })
    .await
    .map_err(|_| "recall_embedding_model_read_failed")??;
    if hash != expected {
        return Err("recall_embedding_model_checksum_failed".into());
    }
    Ok(())
}

#[cfg(all(test, unix))]
#[path = "local/tests.rs"]
mod tests;
