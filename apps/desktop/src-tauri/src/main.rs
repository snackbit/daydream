// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let mut args = std::env::args_os();
    let _binary = args.next();
    let mode = args.next();
    #[cfg(target_os = "linux")]
    if mode.as_deref() == Some(std::ffi::OsStr::new("--daydream-media-worker")) {
        let control = args
            .next()
            .and_then(|v| v.to_str().and_then(|s| s.parse::<i32>().ok()));
        let frames = args
            .next()
            .and_then(|v| v.to_str().and_then(|s| s.parse::<i32>().ok()));
        std::process::exit(match (control, frames) {
            (Some(control), Some(frames)) => {
                daydream_desktop_lib::run_media_worker_process(control, frames)
            }
            _ => 2,
        });
    }
    if mode.as_deref() == Some(std::ffi::OsStr::new("--daydream-whisper-worker")) {
        let Some(app_data_dir) = args.next() else {
            eprintln!("daydream whisper worker requires an app data directory argument");
            std::process::exit(1);
        };
        std::process::exit(daydream_desktop_lib::run_whisper_worker_process(
            app_data_dir.into(),
        ));
    }

    daydream_desktop_lib::run()
}
