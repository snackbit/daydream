use std::{
    collections::VecDeque,
    env,
    fs::{self, File, OpenOptions},
    io::{self, BufRead, BufReader, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex, OnceLock},
    time::{Duration, SystemTime},
};

use serde::Serialize;
use tauri::{AppHandle, Manager};
use tracing_subscriber::{
    filter::{EnvFilter, LevelFilter},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    Layer,
};

const DEFAULT_FILTER: &str = "daydream_desktop_lib=info,warn";
pub(crate) mod failures;
#[cfg(test)]
pub(crate) mod test_support;
const LOG_FILE_NAME: &str = "daydream.log";
const LOG_RETENTION_DAYS: u64 = 7;
const LOG_ROLLOVER_AFTER: Duration = Duration::from_secs(24 * 60 * 60);
const DEBUG_BUNDLE_LINE_LIMIT: usize = 500;

static TELEMETRY: OnceLock<TelemetryRuntime> = OnceLock::new();

#[derive(Debug)]
struct TelemetryRuntime {
    file_sink: Arc<Mutex<Option<File>>>,
    file_path: PathBuf,
    filter: String,
    _file_guard: tracing_appender::non_blocking::WorkerGuard,
    _stdout_guard: tracing_appender::non_blocking::WorkerGuard,
}

#[derive(Clone, Debug)]
struct ResettableLogWriter(Arc<Mutex<Option<File>>>);
impl Write for ResettableLogWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let mut file = self
            .0
            .lock()
            .map_err(|_| io::Error::other("Log writer unavailable"))?;
        match file.as_mut() {
            Some(file) => file.write(bytes),
            None => Ok(bytes.len()),
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        let mut file = self
            .0
            .lock()
            .map_err(|_| io::Error::other("Log writer unavailable"))?;
        match file.as_mut() {
            Some(file) => file.flush(),
            None => Ok(()),
        }
    }
}

pub(crate) fn close_file_for_factory_reset() -> Result<(), String> {
    if let Some(runtime) = TELEMETRY.get() {
        let mut sink = runtime
            .file_sink
            .lock()
            .map_err(|_| "Unable to close the log file safely.")?;
        if let Some(file) = sink.take() {
            file.sync_all()
                .map_err(|_| "Unable to flush the log file safely.")?;
        }
    }
    Ok(())
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DebugLoggingDiagnostics {
    pub enabled: bool,
    pub file_path: Option<String>,
    pub filter: String,
    pub format: String,
    pub latest_file_bytes: Option<u64>,
    pub retention_days: u64,
    pub stdout: bool,
}

pub fn initialize(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    if TELEMETRY.get().is_some() {
        return Ok(());
    }

    let log_dir = app
        .path()
        .app_log_dir()
        .or_else(|_error| app.path().app_data_dir().map(|path| path.join("logs")))?;
    fs::create_dir_all(&log_dir)?;
    let file_path = log_dir.join(LOG_FILE_NAME);
    rotate_stale_log_file(&file_path)?;
    prune_old_logs(&log_dir)?;

    let filter = configured_filter();
    let file = open_log_file(&file_path)?;
    let file_sink = Arc::new(Mutex::new(Some(file)));
    let (file_writer, file_guard) =
        tracing_appender::non_blocking(ResettableLogWriter(file_sink.clone()));
    let (stdout_writer, stdout_guard) = tracing_appender::non_blocking(io::stdout());
    let stdout_filter = env_filter(&filter);
    let file_filter = env_filter(&filter);
    let stdout_layer = tracing_subscriber::fmt::layer()
        .compact()
        .with_ansi(true)
        .with_writer(stdout_writer)
        .with_filter(stdout_filter);
    let file_layer = tracing_subscriber::fmt::layer()
        .json()
        .with_ansi(false)
        .with_writer(file_writer)
        .with_filter(file_filter);

    tracing_subscriber::registry()
        .with(stdout_layer)
        .with(file_layer)
        .try_init()?;
    failures::install_panic_logging();

    let runtime = TelemetryRuntime {
        file_sink,
        file_path,
        filter,
        _file_guard: file_guard,
        _stdout_guard: stdout_guard,
    };
    let _ = TELEMETRY.set(runtime);

    tracing::info!(
        target: "daydream_desktop_lib::telemetry",
        log_file_path = %diagnostics().file_path.unwrap_or_default(),
        filter = %diagnostics().filter,
        "telemetry_initialized"
    );

    Ok(())
}

pub fn diagnostics() -> DebugLoggingDiagnostics {
    match TELEMETRY.get() {
        Some(runtime) => DebugLoggingDiagnostics {
            enabled: true,
            file_path: Some(path_to_string(&runtime.file_path)),
            filter: runtime.filter.clone(),
            format: "stdout=compact,file=jsonl".to_string(),
            latest_file_bytes: fs::metadata(&runtime.file_path)
                .ok()
                .map(|metadata| metadata.len()),
            retention_days: LOG_RETENTION_DAYS,
            stdout: true,
        },
        None => DebugLoggingDiagnostics {
            enabled: false,
            file_path: None,
            filter: configured_filter(),
            format: "stdout=compact,file=jsonl".to_string(),
            latest_file_bytes: None,
            retention_days: LOG_RETENTION_DAYS,
            stdout: false,
        },
    }
}

pub fn read_debug_log_tail() -> Result<Vec<String>, String> {
    let Some(runtime) = TELEMETRY.get() else {
        return Ok(Vec::new());
    };

    read_tail_lines(&runtime.file_path, DEBUG_BUNDLE_LINE_LIMIT).map_err(|error| {
        format!(
            "Unable to read debug log file {}: {error}",
            path_to_string(&runtime.file_path)
        )
    })
}

fn configured_filter() -> String {
    env::var("RUST_LOG").unwrap_or_else(|_error| DEFAULT_FILTER.to_string())
}

fn env_filter(filter: &str) -> EnvFilter {
    EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .parse_lossy(filter)
}

fn open_log_file(path: &Path) -> io::Result<File> {
    OpenOptions::new().append(true).create(true).open(path)
}

fn rotate_stale_log_file(path: &Path) -> io::Result<()> {
    let Ok(metadata) = fs::metadata(path) else {
        return Ok(());
    };
    let Ok(modified_at) = metadata.modified() else {
        return Ok(());
    };
    let Ok(age) = SystemTime::now().duration_since(modified_at) else {
        return Ok(());
    };
    if age < LOG_ROLLOVER_AFTER {
        return Ok(());
    }

    let timestamp = modified_at
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    let archived = path.with_file_name(format!("daydream-{timestamp}.log"));
    fs::rename(path, archived)
}

fn prune_old_logs(log_dir: &Path) -> io::Result<()> {
    let cutoff = SystemTime::now()
        .checked_sub(Duration::from_secs(LOG_RETENTION_DAYS * 24 * 60 * 60))
        .unwrap_or(SystemTime::UNIX_EPOCH);

    for entry in fs::read_dir(log_dir)? {
        let entry = entry?;
        let path = entry.path();
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !file_name.starts_with("daydream-") || !file_name.ends_with(".log") {
            continue;
        }
        let Ok(modified_at) = entry.metadata().and_then(|metadata| metadata.modified()) else {
            continue;
        };
        if modified_at < cutoff {
            let _ = fs::remove_file(path);
        }
    }

    Ok(())
}

fn read_tail_lines(path: &Path, limit: usize) -> io::Result<Vec<String>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut lines = VecDeque::with_capacity(limit);

    for line in reader.lines() {
        if lines.len() == limit {
            lines.pop_front();
        }
        lines.push_back(line?);
    }

    Ok(lines.into_iter().collect())
}

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

#[cfg(test)]
mod tests {
    use super::{env_filter, read_tail_lines};

    #[test]
    fn env_filter_accepts_default_directive() {
        let filter = env_filter("daydream_desktop_lib=info,warn");

        assert!(filter.to_string().contains("daydream_desktop_lib=info"));
    }

    #[test]
    fn tail_reader_returns_bounded_suffix() {
        let path =
            std::env::temp_dir().join(format!("daydream-tail-test-{}.log", std::process::id()));
        std::fs::write(&path, "one\ntwo\nthree\n").expect("write test log");

        let lines = read_tail_lines(&path, 2).expect("read tail");

        let _ = std::fs::remove_file(path);
        assert_eq!(lines, vec!["two".to_string(), "three".to_string()]);
    }
}
