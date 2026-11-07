mod commands;
mod dto;
// The error taxonomy is the IPC contract for every fallible command. The only
// command today (`get_app_info`) cannot fail, so nothing constructs these types
// outside tests yet. Remove this allow as soon as the first fallible command
// lands (Backlog B-002) — it must not outlive the first real use.
#[allow(dead_code)]
mod error;
mod ipc;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![commands::app::get_app_info])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
