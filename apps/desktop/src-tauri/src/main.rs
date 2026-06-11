// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let mut args = std::env::args_os();
    let _binary = args.next();
    if args.next().as_deref() == Some(std::ffi::OsStr::new("--daydream-whisper-worker")) {
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
