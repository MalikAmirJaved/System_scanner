use crate::dto::system_snapshot::SystemSnapshotDto;
use crate::error::{AppError, ErrorCode};
use crate::ipc::CommandResult;

/// Collects a read-only snapshot of OS, CPU, memory, disks, and adapters.
///
/// Takes no arguments, so there is no IPC input to trust. Blocking Win32 work
/// runs on a blocking thread and never on the async reactor.
#[tauri::command]
pub async fn get_system_snapshot() -> CommandResult<SystemSnapshotDto> {
    match tauri::async_runtime::spawn_blocking(collect_platform).await {
        Ok(Ok(snapshot)) => CommandResult::ok(snapshot),
        Ok(Err(error)) => CommandResult::err(error),
        // The join error text can contain a panic message; do not forward it.
        Err(_) => CommandResult::err(
            AppError::new(ErrorCode::Internal).with_detail("snapshot task did not complete"),
        ),
    }
}

#[cfg(windows)]
fn collect_platform() -> Result<SystemSnapshotDto, AppError> {
    use crate::adapters::windows::system_info::WindowsSystemInfo;
    use crate::domain::system::collect_snapshot;

    let captured_at = super::now_ms()?;
    let snapshot = collect_snapshot(&WindowsSystemInfo, captured_at);
    Ok(SystemSnapshotDto::from(&snapshot))
}

#[cfg(not(windows))]
fn collect_platform() -> Result<SystemSnapshotDto, AppError> {
    Err(AppError::new(ErrorCode::Unsupported))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::system_snapshot::SNAPSHOT_SCHEMA_VERSION;

    #[cfg(windows)]
    #[test]
    fn native_snapshot_is_collected_end_to_end() {
        let snapshot = collect_platform().expect("snapshot collection must succeed");

        assert!(snapshot.captured_at > 0);
        assert_eq!(snapshot.schema_version, SNAPSHOT_SCHEMA_VERSION);
        assert!(snapshot.os.is_some(), "OS info should be available");
        assert!(snapshot.memory.is_some(), "memory should be available");
        assert!(
            snapshot
                .disks
                .as_ref()
                .is_some_and(|disks| !disks.is_empty()),
            "at least one disk should be reported"
        );

        for item in snapshot.unavailable {
            assert!(!item.field.is_empty());
            assert!(!item.reason.is_empty());
        }
    }

    #[cfg(not(windows))]
    #[test]
    fn snapshot_is_unsupported_off_windows() {
        let error = collect_platform().expect_err("non-Windows must not collect OS data");
        assert_eq!(error.code, ErrorCode::Unsupported);
    }
}
