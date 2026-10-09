//! Own HTTP sockets and subprocesses until their resources are actually closed.
use crate::maintenance::runtime::{RuntimeActivityLease, RuntimeActivityRegistry};
use std::{
    collections::HashMap,
    io,
    net::{Shutdown, TcpStream},
    process::{Child, ChildStdout, Command, ExitStatus},
    sync::{Arc, Condvar, Mutex},
    thread,
    time::{Duration, Instant},
};

#[derive(Clone, Default)]
pub(super) struct PlaybackResources {
    inner: Arc<(Mutex<State>, Condvar)>,
}
#[derive(Default)]
struct State {
    next: u64,
    entries: HashMap<u64, Entry>,
    registry: Option<RuntimeActivityRegistry>,
}
struct Entry {
    manifests: Vec<std::path::PathBuf>,
    admission: Option<RuntimeActivityLease>,
    orphaned: bool,
    session: Option<String>,
    socket: Option<TcpStream>,
    child: Option<Arc<Mutex<Child>>>,
}
pub(super) struct ResourceLease {
    resources: PlaybackResources,
    id: u64,
}
impl PlaybackResources {
    pub(super) fn attach_registry(&self, registry: RuntimeActivityRegistry) {
        if let Ok(mut state) = self.inner.0.lock() {
            state.registry = Some(registry);
        }
    }
    pub(super) fn admit(&self) -> Result<Option<RuntimeActivityLease>, String> {
        let state = self
            .inner
            .0
            .lock()
            .map_err(|_| "Playback resources unavailable.")?;
        state
            .registry
            .as_ref()
            .map(|r| r.begin("playback"))
            .transpose()
    }
    pub(super) fn begin(
        &self,
        socket: Option<&TcpStream>,
        session: Option<String>,
    ) -> Result<ResourceLease, String> {
        let admission = self.admit()?;
        let socket = socket
            .map(TcpStream::try_clone)
            .transpose()
            .map_err(|_| "Playback socket unavailable.")?;
        let mut state = self
            .inner
            .0
            .lock()
            .map_err(|_| "Playback resources unavailable.")?;
        state.next += 1;
        let id = state.next;
        state.entries.insert(
            id,
            Entry {
                manifests: vec![],
                admission,
                orphaned: false,
                session,
                socket,
                child: None,
            },
        );
        Ok(ResourceLease {
            resources: self.clone(),
            id,
        })
    }
    pub(super) fn cancel_session(&self, session: &str) {
        if let Ok(mut state) = self.inner.0.lock() {
            for entry in state
                .entries
                .values_mut()
                .filter(|e| e.session.as_deref() == Some(session))
            {
                cancel(entry);
            }
        }
    }
    pub(super) fn cancel_and_drain(&self, timeout: Duration) -> Result<(), String> {
        let deadline = Instant::now() + timeout;
        let mut state = self
            .inner
            .0
            .lock()
            .map_err(|_| "Playback resources unavailable.")?;
        for entry in state.entries.values_mut() {
            cancel(entry);
        }
        while !state.entries.is_empty() {
            state.entries.retain(|_, entry| {
                !(entry.orphaned
                    && entry.child.as_ref().is_some_and(|child| {
                        child
                            .lock()
                            .is_ok_and(|mut child| child.try_wait().ok().flatten().is_some())
                    }))
            });
            if state.entries.is_empty() {
                break;
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err("Playback readers did not close. Maintenance remains blocked.".into());
            }
            state = self
                .inner
                .1
                .wait_timeout(state, remaining.min(Duration::from_millis(50)))
                .map_err(|_| "Playback resources unavailable.")?
                .0;
        }
        Ok(())
    }
}
fn cancel(entry: &mut Entry) {
    if let Some(socket) = &entry.socket {
        let _ = socket.shutdown(Shutdown::Both);
    }
    if let Some(child) = &entry.child {
        if let Ok(mut child) = child.lock() {
            let _ = child.kill();
        }
    }
}
impl ResourceLease {
    pub(super) fn manifest(&self, contents: &str) -> io::Result<std::path::PathBuf> {
        use std::io::Write;
        let path = std::env::temp_dir().join(format!(
            "daydream-playback-{}.ffconcat",
            uuid::Uuid::now_v7()
        ));
        let mut options = std::fs::OpenOptions::new();
        options.create_new(true).write(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut state = self
            .resources
            .inner
            .0
            .lock()
            .map_err(|_| io::Error::other("Playback resources unavailable."))?;
        let entry = state
            .entries
            .get_mut(&self.id)
            .ok_or_else(|| io::Error::other("Playback request cancelled."))?;
        let mut file = options.open(&path)?;
        entry.manifests.push(path.clone());
        file.write_all(contents.as_bytes())?;
        Ok(path)
    }
    pub(super) fn set_session(&self, session: &str) {
        if let Ok(mut state) = self.resources.inner.0.lock() {
            if let Some(entry) = state.entries.get_mut(&self.id) {
                entry.session = Some(session.into());
            }
        }
    }
    pub(super) fn spawn(&self, command: &mut Command) -> io::Result<ChildStdout> {
        let mut state = self
            .resources
            .inner
            .0
            .lock()
            .map_err(|_| io::Error::other("Playback resources unavailable."))?;
        let entry = state
            .entries
            .get_mut(&self.id)
            .ok_or_else(|| io::Error::other("Playback request cancelled."))?;
        if entry
            .admission
            .as_ref()
            .is_some_and(|lease| lease.cancellation().is_cancelled())
        {
            return Err(io::Error::other("Playback request cancelled."));
        }
        let mut child = command.spawn()?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| io::Error::other("Playback output unavailable."))?;
        entry.child = Some(Arc::new(Mutex::new(child)));
        Ok(stdout)
    }
    pub(super) fn kill(&self) {
        if let Ok(mut state) = self.resources.inner.0.lock() {
            if let Some(entry) = state.entries.get_mut(&self.id) {
                cancel(entry);
            }
        }
    }
    pub(super) fn wait(&self) -> io::Result<ExitStatus> {
        let child = self
            .resources
            .inner
            .0
            .lock()
            .map_err(|_| io::Error::other("Playback resources unavailable."))?
            .entries
            .get(&self.id)
            .and_then(|entry| entry.child.clone())
            .ok_or_else(|| io::Error::other("Playback child unavailable."))?;
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            if let Some(status) = child
                .lock()
                .map_err(|_| io::Error::other("Playback child unavailable."))?
                .try_wait()?
            {
                return Ok(status);
            }
            if Instant::now() >= deadline {
                self.kill();
                return Err(io::Error::other("Playback child did not exit."));
            }
            thread::sleep(Duration::from_millis(10));
        }
    }
}
impl Drop for Entry {
    fn drop(&mut self) {
        for path in &self.manifests {
            let _ = std::fs::remove_file(path);
        }
    }
}
impl Drop for ResourceLease {
    fn drop(&mut self) {
        self.kill();
        let _ = self.wait();
        let reaped = {
            let state = self.resources.inner.0.lock();
            match state {
                Ok(state) => state
                    .entries
                    .get(&self.id)
                    .and_then(|entry| entry.child.as_ref())
                    .is_none_or(|child| {
                        child
                            .lock()
                            .is_ok_and(|mut child| child.try_wait().ok().flatten().is_some())
                    }),
                Err(_) => false,
            }
        };
        if reaped {
            if let Ok(mut state) = self.resources.inner.0.lock() {
                state.entries.remove(&self.id);
                self.resources.inner.1.notify_all();
            }
        } else if let Ok(mut state) = self.resources.inner.0.lock() {
            if let Some(entry) = state.entries.get_mut(&self.id) {
                entry.orphaned = true;
            }
        }
    }
}
