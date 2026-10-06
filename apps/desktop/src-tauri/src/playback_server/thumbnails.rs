use std::{
    collections::{HashMap, HashSet, VecDeque},
    process::Command,
    sync::{mpsc, Arc, Condvar, Mutex},
    thread,
};

use crate::ffmpeg;

use super::{ffmpeg_args::thumbnail_ffmpeg_args, PlaybackSegment, PlaybackSession};

pub(super) const THUMBNAIL_WORKER_COUNT: usize = 2;
const THUMBNAIL_INTERVAL_MS: i64 = 3_000;

#[derive(Clone)]
pub(super) struct ThumbnailService {
    sessions: Arc<Mutex<HashMap<String, PlaybackSession>>>,
    state: Arc<(Mutex<ThumbnailState>, Condvar)>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) struct ThumbnailKey {
    pub(super) video_stream_index: i64,
    pub(super) offset_ms: i64,
    pub(super) segment_id: String,
    pub(super) session_id: String,
}

pub(super) struct ThumbnailJob {
    pub(super) key: ThumbnailKey,
    pub(super) offset_ms: i64,
    pub(super) segment: PlaybackSegment,
    pub(super) video_stream_index: i64,
}

pub(super) struct ThumbnailState {
    pub(super) cache: HashMap<ThumbnailKey, Arc<Vec<u8>>>,
    pub(super) in_progress: HashMap<ThumbnailKey, Vec<mpsc::Sender<ThumbnailResult>>>,
    pub(super) queue: VecDeque<ThumbnailJob>,
    queued_keys: HashSet<ThumbnailKey>,
    running_keys: HashSet<ThumbnailKey>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum ThumbnailError {
    Failed,
    StaleSession,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum ThumbnailEnqueueOutcome {
    Cached,
    Deduplicated,
    Enqueued,
}

pub(super) type ThumbnailResult = Result<Arc<Vec<u8>>, ThumbnailError>;
impl ThumbnailService {
    pub(super) fn new(
        sessions: Arc<Mutex<HashMap<String, PlaybackSession>>>,
        worker_count: usize,
    ) -> Self {
        let service = Self {
            sessions,
            state: Arc::new((Mutex::new(ThumbnailState::new()), Condvar::new())),
        };
        for worker_index in 0..worker_count {
            let service_for_worker = service.clone();
            let _ = thread::Builder::new()
                .name(format!("daydream-thumbnail-worker-{worker_index}"))
                .spawn(move || service_for_worker.worker_loop());
        }

        service
    }

    pub(super) fn request_thumbnail(
        &self,
        key: ThumbnailKey,
        segment: PlaybackSegment,
        video_stream_index: i64,
        offset_ms: i64,
    ) -> ThumbnailResult {
        let (sender, receiver) = mpsc::channel();
        let job = ThumbnailJob {
            key: key.clone(),
            offset_ms,
            segment,
            video_stream_index,
        };
        let (lock, condvar) = &*self.state;
        {
            let mut state = lock.lock().map_err(|_error| ThumbnailError::Failed)?;
            if let ThumbnailEnqueueOutcome::Cached = state.enqueue(key, job, sender)? {
                return receiver.recv().unwrap_or(Err(ThumbnailError::Failed));
            }
        }
        condvar.notify_one();

        receiver.recv().unwrap_or(Err(ThumbnailError::Failed))
    }

    pub(super) fn close_session(&self, session_id: &str) {
        let waiters = {
            let (lock, condvar) = &*self.state;
            let Ok(mut state) = lock.lock() else {
                return;
            };
            let waiters = state.close_session(session_id);
            condvar.notify_all();
            waiters
        };
        for waiter in waiters {
            let _ = waiter.send(Err(ThumbnailError::StaleSession));
        }
    }

    fn worker_loop(&self) {
        loop {
            let job = {
                let (lock, condvar) = &*self.state;
                let mut state = match lock.lock() {
                    Ok(state) => state,
                    Err(_) => return,
                };
                loop {
                    if let Some(job) = state.pop_next_job() {
                        break job;
                    }
                    state = match condvar.wait(state) {
                        Ok(state) => state,
                        Err(_) => return,
                    };
                }
            };

            let result = if self.session_exists(&job.key.session_id) {
                generate_thumbnail(&job.segment, job.video_stream_index, job.offset_ms)
                    .map(Arc::new)
                    .map_err(|_error| ThumbnailError::Failed)
            } else {
                Err(ThumbnailError::StaleSession)
            };
            let result = if result.is_ok() && !self.session_exists(&job.key.session_id) {
                Err(ThumbnailError::StaleSession)
            } else {
                result
            };
            self.complete_job(job.key, result);
        }
    }

    fn complete_job(&self, key: ThumbnailKey, result: ThumbnailResult) {
        let waiters = {
            let (lock, _condvar) = &*self.state;
            let Ok(mut state) = lock.lock() else {
                return;
            };
            state.complete_job(key, result.clone())
        };
        for waiter in waiters {
            let _ = waiter.send(result.clone());
        }
    }

    fn session_exists(&self, session_id: &str) -> bool {
        self.sessions
            .lock()
            .is_ok_and(|sessions| sessions.contains_key(session_id))
    }
}

impl ThumbnailState {
    pub(super) fn new() -> Self {
        Self {
            cache: HashMap::new(),
            in_progress: HashMap::new(),
            queue: VecDeque::new(),
            queued_keys: HashSet::new(),
            running_keys: HashSet::new(),
        }
    }

    pub(super) fn enqueue(
        &mut self,
        key: ThumbnailKey,
        job: ThumbnailJob,
        sender: mpsc::Sender<ThumbnailResult>,
    ) -> Result<ThumbnailEnqueueOutcome, ThumbnailError> {
        if let Some(cached) = self.cache.get(&key) {
            sender
                .send(Ok(Arc::clone(cached)))
                .map_err(|_error| ThumbnailError::Failed)?;
            return Ok(ThumbnailEnqueueOutcome::Cached);
        }
        if let Some(waiters) = self.in_progress.get_mut(&key) {
            waiters.push(sender);
            return Ok(ThumbnailEnqueueOutcome::Deduplicated);
        }

        self.in_progress.insert(key.clone(), vec![sender]);
        self.queued_keys.insert(key);
        self.queue.push_back(job);
        Ok(ThumbnailEnqueueOutcome::Enqueued)
    }

    pub(super) fn pop_next_job(&mut self) -> Option<ThumbnailJob> {
        let job = self.queue.pop_front()?;
        self.queued_keys.remove(&job.key);
        self.running_keys.insert(job.key.clone());
        Some(job)
    }

    fn complete_job(
        &mut self,
        key: ThumbnailKey,
        result: ThumbnailResult,
    ) -> Vec<mpsc::Sender<ThumbnailResult>> {
        self.running_keys.remove(&key);
        if let Ok(bytes) = &result {
            self.cache.insert(key.clone(), Arc::clone(bytes));
        }
        self.in_progress.remove(&key).unwrap_or_default()
    }

    pub(super) fn close_session(&mut self, session_id: &str) -> Vec<mpsc::Sender<ThumbnailResult>> {
        self.cache
            .retain(|key, _bytes| key.session_id.as_str() != session_id);
        let mut removed_queued_keys = HashSet::new();
        self.queue.retain(|job| {
            let keep = job.key.session_id != session_id;
            if !keep {
                removed_queued_keys.insert(job.key.clone());
                self.queued_keys.remove(&job.key);
            }
            keep
        });

        let mut waiters = Vec::new();
        for key in removed_queued_keys {
            if let Some(mut next_waiters) = self.in_progress.remove(&key) {
                waiters.append(&mut next_waiters);
            }
        }

        waiters
    }
}
fn generate_thumbnail(
    segment: &PlaybackSegment,
    video_stream_index: i64,
    offset_ms: i64,
) -> Result<Vec<u8>, String> {
    let output = Command::new(ffmpeg::binary_path())
        .args(thumbnail_ffmpeg_args(
            segment,
            video_stream_index,
            offset_ms,
        ))
        .output()
        .map_err(|error| error.to_string())?;

    if output.status.success() && !output.stdout.is_empty() {
        Ok(output.stdout)
    } else {
        Err("Unable to generate thumbnail".to_string())
    }
}

pub(super) fn thumbnail_offsets(duration_ms: i64) -> Vec<i64> {
    let mut offsets = Vec::new();
    let mut offset_ms = 0;
    while offset_ms < duration_ms.max(1) {
        offsets.push(offset_ms);
        offset_ms += THUMBNAIL_INTERVAL_MS;
    }
    offsets
}
