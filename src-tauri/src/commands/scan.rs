use std::sync::Arc;

use tauri::{AppHandle, Emitter, Manager, State};

use crate::app::jobs::{EventSink, Supervisor};
use crate::dto::scan::{CancelScanDto, ScanStatusDto, StartScanDto, StartScanRequest};
use crate::error::{AppError, ErrorCode};
use crate::ipc::CommandResult;

/// Starts one read-only scan and returns its job ID immediately.
///
/// The request is untrusted: shape is checked by `StartScanRequest::parse` and
/// the roots by the traversal validator, both before any filesystem access.
/// Progress and the terminal outcome arrive as `scan:*` events; the
/// authoritative state is `get_scan_status`. Only counters are returned.
#[tauri::command]
pub async fn start_scan(app: AppHandle, request: StartScanRequest) -> CommandResult<StartScanDto> {
    // Fetched from the handle: an async command cannot borrow `State`.
    let supervisor = app.state::<Supervisor>().inner().clone();
    let result = run_start_scan(app, supervisor, request).await;
    result.log_outcome("start_scan")
}

async fn run_start_scan(
    app: AppHandle,
    supervisor: Supervisor,
    request: StartScanRequest,
) -> CommandResult<StartScanDto> {
    let parsed = match request.parse() {
        Ok(parsed) => parsed,
        Err(message) => {
            return CommandResult::err(AppError::new(ErrorCode::InvalidInput).with_detail(message));
        }
    };

    let sink: EventSink = Arc::new(move |event| {
        if app.emit(event.name(), event.payload()).is_err() {
            tracing::warn!(event = event.name(), "scan event could not be delivered");
        }
    });

    // Starting stats each root, which can be slow on a dead network mapping, so
    // it stays off the async executor.
    let started = tauri::async_runtime::spawn_blocking(move || {
        supervisor.start(parsed.roots, parsed.max_entries, sink)
    })
    .await;

    match started {
        Ok(Ok(job_id)) => CommandResult::ok(StartScanDto { job_id }),
        Ok(Err(error)) => CommandResult::err(error),
        Err(_) => CommandResult::err(
            AppError::new(ErrorCode::Internal).with_detail("scan start did not complete"),
        ),
    }
}

/// Requests cooperative cancellation. Idempotent: `accepted` is `false` when
/// there is nothing to cancel.
#[tauri::command]
pub fn cancel_scan(
    supervisor: State<'_, Supervisor>,
    job_id: Option<String>,
) -> CommandResult<CancelScanDto> {
    let accepted = supervisor.cancel(job_id.as_deref());
    CommandResult::ok(CancelScanDto { accepted }).log_outcome("cancel_scan")
}

/// Authoritative state of the current or most recent scan. Does not touch the
/// disk.
#[tauri::command]
pub fn get_scan_status(
    supervisor: State<'_, Supervisor>,
    job_id: Option<String>,
) -> CommandResult<ScanStatusDto> {
    supervisor.status(job_id.as_deref()).into()
}
