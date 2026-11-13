mod adapters;
mod commands;
mod domain;
mod dto;
mod error;
mod ipc;
mod ports;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::app::get_app_info,
            commands::processes::list_processes,
            commands::system::get_system_snapshot
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
