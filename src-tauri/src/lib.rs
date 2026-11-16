mod adapters;
mod commands;
mod domain;
mod dto;
mod error;
mod ipc;
mod logging;
mod ports;
mod redact;
mod startup;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    if let Err(error) = tauri::Builder::default()
        .setup(|app| {
            use tauri::Manager;

            match app.path().app_log_dir() {
                Ok(dir) => match logging::init(&dir) {
                    Ok(path) => tracing::info!(
                        version = env!("CARGO_PKG_VERSION"),
                        log = %path.display(),
                        "system-scanner starting"
                    ),
                    Err(error) => eprintln!("logging unavailable: {error}"),
                },
                Err(error) => eprintln!("log directory unavailable: {error}"),
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app::get_app_info,
            commands::diagnostics::get_diagnostics,
            commands::processes::list_processes,
            commands::system::get_system_snapshot
        ])
        .run(tauri::generate_context!())
    {
        // No window, no IPC, possibly no log. This is the only user-visible
        // path left, and it must not panic (see `startup.rs`).
        startup::report_fatal(&error.to_string());
    }
}
