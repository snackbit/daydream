use std::{env, path::PathBuf};

use serde::Serialize;
use tauri::{Manager, State};

use crate::recorder::{
    self, ffmpeg, ffmpeg::FfmpegPathSource, RecorderBackendHealth, RecorderFfmpegCapabilities,
};
use crate::storage::{collect_database_diagnostics, AppStorage, DatabaseDiagnostics};

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
    database: DatabaseDiagnostics,
    ffmpeg: FfmpegDiagnostics,
    recorder: RecorderBackendHealth,
    storage: StorageDiagnostics,
    system: SystemDiagnostics,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FfmpegDiagnostics {
    available: bool,
    capabilities: RecorderFfmpegCapabilities,
    error: Option<String>,
    path: Option<String>,
    source: FfmpegSource,
    version_line: Option<String>,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum FfmpegSource {
    DaydreamFfmpegEnv,
    Path,
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
pub async fn get_diagnostics(
    app: tauri::AppHandle,
    storage: State<'_, AppStorage>,
) -> Result<DaydreamDiagnostics, String> {
    Ok(DaydreamDiagnostics {
        app: AppDiagnostics {
            identifier: app.config().identifier.to_string(),
            name: "Daydream".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
        database: collect_database_diagnostics(&storage),
        ffmpeg: collect_ffmpeg_diagnostics(),
        recorder: recorder::collect_backend_health().await,
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
    })
}

fn collect_ffmpeg_diagnostics() -> FfmpegDiagnostics {
    let health = ffmpeg::collect_ffmpeg_health();
    let source = ffmpeg::selected_ffmpeg_path_details()
        .map(|selected| source_from_ffmpeg_path_source(selected.source))
        .unwrap_or(FfmpegSource::Unconfigured);

    FfmpegDiagnostics {
        available: health.available,
        capabilities: health.capabilities,
        error: health.error,
        path: health.path,
        source,
        version_line: health.version_line,
    }
}

fn source_from_ffmpeg_path_source(source: FfmpegPathSource) -> FfmpegSource {
    match source {
        FfmpegPathSource::DaydreamFfmpegEnv => FfmpegSource::DaydreamFfmpegEnv,
        FfmpegPathSource::Path => FfmpegSource::Path,
    }
}

fn path_to_string(path: &PathBuf) -> String {
    path.to_string_lossy().to_string()
}

#[cfg(test)]
mod tests {
    use crate::recorder::ffmpeg::{parse_capabilities, FfmpegPathSource};

    use super::source_from_ffmpeg_path_source;

    #[test]
    fn parses_ffmpeg_capability_summary() {
        let capabilities = parse_capabilities(
            " D  lavfi\n DE pulse\n",
            " DE rawvideo\n  E matroska\n",
            " V....D libx264\n A..... libopus\n",
        );

        assert!(capabilities.rawvideo_demuxer);
        assert!(capabilities.pulse_input);
        assert!(capabilities.matroska_muxer);
        assert!(capabilities.libx264_encoder);
        assert!(capabilities.libopus_encoder);
    }

    #[test]
    fn reports_path_sourced_ffmpeg_distinctly() {
        let source = source_from_ffmpeg_path_source(FfmpegPathSource::Path);

        assert_eq!(
            serde_json::to_value(source).expect("serialize ffmpeg source"),
            serde_json::json!("path"),
        );
    }

    #[test]
    fn reports_env_sourced_ffmpeg_distinctly() {
        let source = source_from_ffmpeg_path_source(FfmpegPathSource::DaydreamFfmpegEnv);

        assert_eq!(
            serde_json::to_value(source).expect("serialize ffmpeg source"),
            serde_json::json!("daydream-ffmpeg-env"),
        );
    }
}
