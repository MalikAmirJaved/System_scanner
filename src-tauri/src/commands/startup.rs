use crate::dto::startup_inventory::StartupInventoryDto;
use crate::error::{AppError, ErrorCode};
use crate::ipc::CommandResult;

/// Returns the startup-entry and service inventory.
///
/// The command takes no input, so there is nothing to validate. Both groups are
/// read-only observations; a source the current user cannot read is reported in
/// `unavailable` rather than dropped. Enumeration runs on a blocking thread.
#[tauri::command]
pub async fn get_startup_inventory() -> CommandResult<StartupInventoryDto> {
    run_startup_inventory()
        .await
        .log_outcome("get_startup_inventory")
}

async fn run_startup_inventory() -> CommandResult<StartupInventoryDto> {
    match tauri::async_runtime::spawn_blocking(collect_platform).await {
        Ok(Ok(inventory)) => CommandResult::ok(inventory),
        Ok(Err(error)) => CommandResult::err(error),
        // The join error text can contain a panic message; do not forward it.
        Err(_) => CommandResult::err(
            AppError::new(ErrorCode::Internal)
                .with_detail("startup inventory task did not complete"),
        ),
    }
}

#[cfg(windows)]
fn collect_platform() -> Result<StartupInventoryDto, AppError> {
    use crate::adapters::windows::startup::WindowsStartup;
    use crate::domain::startup::collect_inventory;

    let captured_at = super::now_ms()?;
    let inventory = collect_inventory(&WindowsStartup);
    Ok(StartupInventoryDto::from_inventory(&inventory, captured_at))
}

#[cfg(not(windows))]
fn collect_platform() -> Result<StartupInventoryDto, AppError> {
    Err(AppError::new(ErrorCode::Unsupported))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    #[test]
    fn native_inventory_reports_every_source_and_a_capture_time() {
        use crate::dto::startup_inventory::STARTUP_INVENTORY_SCHEMA_VERSION;

        let inventory = collect_platform().expect("startup inventory must be collectable");

        assert_eq!(inventory.schema_version, STARTUP_INVENTORY_SCHEMA_VERSION);
        assert!(inventory.captured_at > 0);
        assert_eq!(inventory.startup.len(), 6);
        assert!(inventory.services.is_some() || !inventory.unavailable.is_empty());
    }

    #[cfg(not(windows))]
    #[test]
    fn startup_inventory_is_unsupported_off_windows() {
        let error = collect_platform().expect_err("non-Windows must not read startup entries");
        assert_eq!(error.code, ErrorCode::Unsupported);
    }
}
