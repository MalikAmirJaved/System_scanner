use crate::domain::process::PageRequest;
use crate::dto::process_list::ProcessListDto;
use crate::error::{AppError, ErrorCode};
use crate::ipc::CommandResult;

/// Enumerates processes and returns one page.
///
/// `offset` and `limit` are untrusted IPC input: they are validated here, before
/// any OS work happens. A rejected request returns `invalid_input` and never
/// echoes the value back. Enumeration runs on a blocking thread.
#[tauri::command]
pub async fn list_processes(
    offset: Option<u32>,
    limit: Option<u32>,
) -> CommandResult<ProcessListDto> {
    let result = run_list_processes(offset, limit).await;
    result.log_outcome("list_processes")
}

async fn run_list_processes(
    offset: Option<u32>,
    limit: Option<u32>,
) -> CommandResult<ProcessListDto> {
    let request = match PageRequest::new(offset, limit) {
        Ok(request) => request,
        Err(error) => {
            return CommandResult::err(
                AppError::new(ErrorCode::InvalidInput).with_detail(error.message()),
            );
        }
    };

    match tauri::async_runtime::spawn_blocking(move || collect_platform(request)).await {
        Ok(Ok(list)) => CommandResult::ok(list),
        Ok(Err(error)) => CommandResult::err(error),
        // The join error text can contain a panic message; do not forward it.
        Err(_) => CommandResult::err(
            AppError::new(ErrorCode::Internal).with_detail("process task did not complete"),
        ),
    }
}

#[cfg(windows)]
fn collect_platform(request: PageRequest) -> Result<ProcessListDto, AppError> {
    use crate::adapters::windows::processes::WindowsProcesses;
    use crate::domain::process::collect_page;

    let captured_at = super::now_ms()?;
    let page = collect_page(&WindowsProcesses, request).map_err(super::probe_error)?;
    Ok(ProcessListDto::from_page(&page, captured_at))
}

#[cfg(not(windows))]
fn collect_platform(_request: PageRequest) -> Result<ProcessListDto, AppError> {
    Err(AppError::new(ErrorCode::Unsupported))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paging_input_is_validated_before_collection() {
        assert_eq!(
            PageRequest::new(Some(0), Some(0)).unwrap_err().message(),
            "limit must be at least 1"
        );
        assert_eq!(
            PageRequest::new(Some(0), Some(501)).unwrap_err().message(),
            "limit must be at most 500"
        );
        assert_eq!(
            PageRequest::new(Some(1_000_001), None)
                .unwrap_err()
                .message(),
            "offset must be at most 1000000"
        );
    }

    #[cfg(windows)]
    #[test]
    fn native_page_respects_the_requested_limit() {
        use crate::dto::process_list::PROCESS_LIST_SCHEMA_VERSION;

        let page = collect_platform(PageRequest::new(Some(0), Some(1)).unwrap())
            .expect("process list must be collectable");

        assert_eq!(page.items.len(), 1);
        assert!(page.total >= 1);
        assert_eq!(page.limit, 1);
        assert_eq!(page.offset, 0);
        assert_eq!(page.schema_version, PROCESS_LIST_SCHEMA_VERSION);
        assert!(page.captured_at > 0);
        // Depth is derived before paging, so even a one-row page carries it.
        assert!(page.items[0].depth <= crate::domain::process::MAX_HIERARCHY_DEPTH);
    }

    #[cfg(windows)]
    #[test]
    fn native_page_is_ordered_by_pid() {
        let page = collect_platform(PageRequest::new(None, None).unwrap())
            .expect("process list must be collectable");

        let pids: Vec<u32> = page.items.iter().map(|item| item.pid).collect();
        let mut sorted = pids.clone();
        sorted.sort_unstable();
        assert_eq!(pids, sorted, "paging must stay stable across refreshes");
    }

    #[cfg(not(windows))]
    #[test]
    fn processes_are_unsupported_off_windows() {
        let error = collect_platform(PageRequest::new(None, None).unwrap())
            .expect_err("non-Windows must not enumerate processes");
        assert_eq!(error.code, ErrorCode::Unsupported);
    }
}
