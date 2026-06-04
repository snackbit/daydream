use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
};

use serde::Serialize;
use tauri::Manager;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppDiagnostics {
    identifier: String,
    name: String,
    version: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DaydreamDiagnostics {
    app: AppDiagnostics,
    ffmpeg: FfmpegDiagnostics,
    storage: StorageDiagnostics,
    system: SystemDiagnostics,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FfmpegDiagnostics {
    available: bool,
    error: Option<String>,
    path: Option<String>,
    source: FfmpegSource,
    version_line: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum FfmpegSource {
    DaydreamFfmpegEnv,
    Unconfigured,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageDiagnostics {
    app_data_dir: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemDiagnostics {
    arch: String,
    family: String,
    platform: String,
}

#[tauri::command]
pub fn get_diagnostics(app: tauri::AppHandle) -> DaydreamDiagnostics {
    DaydreamDiagnostics {
        app: AppDiagnostics {
            identifier: app.config().identifier.to_string(),
            name: "Daydream".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
        ffmpeg: collect_ffmpeg_diagnostics(),
        storage: StorageDiagnostics {
            app_data_dir: app
                .path()
                .app_data_dir()
                .ok()
                .map(|path| path_to_string(&path)),
        },
        system: SystemDiagnostics {
            arch: env::consts::ARCH.to_string(),
            family: env::consts::FAMILY.to_string(),
            platform: env::consts::OS.to_string(),
        },
    }
}

fn collect_ffmpeg_diagnostics() -> FfmpegDiagnostics {
    let Ok(path) = env::var("DAYDREAM_FFMPEG_PATH") else {
        return FfmpegDiagnostics {
            available: false,
            error: Some("DAYDREAM_FFMPEG_PATH is not configured.".to_string()),
            path: None,
            source: FfmpegSource::Unconfigured,
            version_line: None,
        };
    };

    let trimmed_path = path.trim();

    if trimmed_path.is_empty() {
        return FfmpegDiagnostics {
            available: false,
            error: Some("DAYDREAM_FFMPEG_PATH is empty.".to_string()),
            path: None,
            source: FfmpegSource::Unconfigured,
            version_line: None,
        };
    }

    match probe_ffmpeg_version_line(Path::new(trimmed_path)) {
        Ok(version_line) => FfmpegDiagnostics {
            available: true,
            error: None,
            path: Some(trimmed_path.to_string()),
            source: FfmpegSource::DaydreamFfmpegEnv,
            version_line: Some(version_line),
        },
        Err(error) => FfmpegDiagnostics {
            available: false,
            error: Some(error),
            path: Some(trimmed_path.to_string()),
            source: FfmpegSource::DaydreamFfmpegEnv,
            version_line: None,
        },
    }
}

fn path_to_string(path: &PathBuf) -> String {
    path.to_string_lossy().to_string()
}

fn probe_ffmpeg_version_line(path: &Path) -> Result<String, String> {
    let output = Command::new(path)
        .arg("-version")
        .output()
        .map_err(|error| format!("Failed to run FFmpeg: {error}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let reason = first_non_empty_line(&stderr).unwrap_or("unknown error");

        return Err(format!("FFmpeg exited with {}: {reason}", output.status));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    first_non_empty_line(&stdout)
        .map(str::to_string)
        .ok_or_else(|| "FFmpeg did not print a version line.".to_string())
}

fn first_non_empty_line(value: &str) -> Option<&str> {
    value.lines().map(str::trim).find(|line| !line.is_empty())
}

#[cfg(test)]
mod tests {
    use super::first_non_empty_line;

    #[test]
    fn finds_first_non_empty_line() {
        assert_eq!(
            first_non_empty_line("\n\nffmpeg version 7.1\nconfiguration"),
            Some("ffmpeg version 7.1"),
        );
    }

    #[test]
    fn returns_none_for_blank_output() {
        assert_eq!(first_non_empty_line(" \n\t\n"), None);
    }
}
