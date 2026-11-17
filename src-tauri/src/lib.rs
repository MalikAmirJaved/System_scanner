mod adapters;
mod app;
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
        .on_window_event(|window, event| {
            // No scan may keep running once its window is gone. The supervisor is
            // created in `setup`, so tolerate it being absent.
            if let tauri::WindowEvent::Destroyed = event {
                use tauri::Manager;
                if let Some(supervisor) = window.app_handle().try_state::<app::jobs::Supervisor>() {
                    supervisor.shutdown();
                }
            }
        })
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
            let mut supervisor = app::jobs::Supervisor::new();
            match app.path().app_data_dir() {
                Ok(directory) => {
                    let path = directory.join(adapters::persistence::DATABASE_FILE_NAME);
                    match adapters::persistence::Database::open(&path) {
                        Ok(database) => {
                            use ports::sessions::ScanSessionStore;

                            let schema_version = database.schema_version().unwrap_or(0);
                            tracing::info!(schema_version, "database ready");

                            // A session still open here belonged to a run that
                            // ended without finishing it (crash or kill).
                            match database.mark_interrupted(app::jobs::unix_ms()) {
                                Ok(0) | Err(_) => {}
                                Ok(count) => tracing::warn!(
                                    count,
                                    "scan sessions from a previous run were marked interrupted"
                                ),
                            }

                            let database = std::sync::Arc::new(database);
                            app.manage(std::sync::Arc::clone(&database));
                            supervisor = supervisor.with_store(database);
                        }
                        Err(error) => tracing::warn!(
                            error = %error,
                            "database unavailable; continuing without persistence"
                        ),
                    }
                }
                Err(error) => tracing::warn!(%error, "data directory unavailable"),
            }
            app.manage(supervisor);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app::get_app_info,
            commands::diagnostics::get_diagnostics,
            commands::processes::list_processes,
            commands::scan::cancel_scan,
            commands::scan::get_scan_status,
            commands::scan::start_scan,
            commands::system::get_system_snapshot
        ])
        .run(tauri::generate_context!())
    {
        // No window, no IPC, possibly no log. This is the only user-visible
        // path left, and it must not panic (see `startup.rs`).
        startup::report_fatal(&error.to_string());
    }
}
