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
                    Ok(_) => tracing::info!(
                        version = env!("CARGO_PKG_VERSION"),
                        log_file = logging::LOG_FILE_NAME,
                        "system-scanner starting"
                    ),
                    Err(error) => eprintln!("logging unavailable: {error}"),
                },
                Err(error) => eprintln!("log directory unavailable: {error}"),
            }

            // Persistence is optional for the current read-only panels. A failure
            // is logged and the app keeps running; nothing here can destroy user
            // data because opening only applies forward migrations.
            match app.path().app_data_dir() {
                Ok(directory) => {
                    let path = directory.join(adapters::persistence::DATABASE_FILE_NAME);
                    match adapters::persistence::Database::open(&path) {
                        Ok(database) => {
                            let schema_version = database.schema_version().unwrap_or(0);
                            tracing::info!(schema_version, "database ready");
                            app.manage(database);
                        }
                        Err(error) => tracing::warn!(
                            error = %error,
                            "database unavailable; continuing without persistence"
                        ),
                    }
                }
                Err(error) => tracing::warn!(%error, "data directory unavailable"),
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
