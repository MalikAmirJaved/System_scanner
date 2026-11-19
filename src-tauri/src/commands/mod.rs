use crate::error::{AppError, ErrorCode};
use crate::ports::ProbeError;

pub mod app;
pub mod diagnostics;
pub mod processes;
pub mod scan;
pub mod startup;
pub mod system;

/// Converts a capability-probe failure into the published IPC error shape.
pub(crate) fn probe_error(error: ProbeError) -> AppError {
    AppError::new(match error {
        ProbeError::AccessDenied => ErrorCode::AccessDenied,
        ProbeError::Failed => ErrorCode::Internal,
    })
}

/// Unix milliseconds UTC, used as the capture timestamp on list responses.
pub(crate) fn now_ms() -> Result<u64, AppError> {
    let elapsed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| AppError::new(ErrorCode::Internal))?;
    u64::try_from(elapsed.as_millis()).map_err(|_| AppError::new(ErrorCode::Internal))
}
