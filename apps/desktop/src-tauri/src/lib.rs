mod diagnostics;
mod storage;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let storage = storage::initialize(app.handle())?;
            app.manage(storage);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![diagnostics::get_diagnostics])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
