use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

static CORRELATION_SEQ: AtomicU64 = AtomicU64::new(1);

/// Stable error codes shared with the frontend (`src/services/ipc.ts`).
///
/// Changing a variant's serialized name is a breaking IPC change and must be
/// documented in `API.md`.
///
/// The whole taxonomy is the published contract even though several variants are
/// not constructed until the features that raise them land. The allowance is
/// scoped to this enum on purpose; it is not a module-wide suppression.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[allow(dead_code)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    InvalidInput,
    NotPermitted,
    AccessDenied,
    NotFound,
    Unsupported,
    Timeout,
    Busy,
    Cancelled,
    DiskFull,
    DbError,
    Internal,
}

impl ErrorCode {
    /// Whether the caller may reasonably retry without changing the request.
    pub const fn retryable(self) -> bool {
        matches!(
            self,
            Self::AccessDenied | Self::NotFound | Self::Timeout | Self::Busy | Self::DbError
        )
    }

    /// A safe, user-facing default message. Never include paths or OS details.
    pub const fn default_message(self) -> &'static str {
        match self {
            Self::InvalidInput => "The request was invalid.",
            Self::NotPermitted => "This operation is not permitted.",
            Self::AccessDenied => "Windows denied access to the requested resource.",
            Self::NotFound => "The requested item no longer exists.",
            Self::Unsupported => "This Windows version does not support the requested operation.",
            Self::Timeout => "The operation took too long and was stopped.",
            Self::Busy => "Another operation is already using this resource.",
            Self::Cancelled => "The operation was cancelled.",
            Self::DiskFull => "There is not enough disk space to complete the operation.",
            Self::DbError => "The local history database could not be read or written.",
            Self::Internal => "An unexpected internal error occurred.",
        }
    }
}

/// The only error shape that crosses the IPC boundary.
///
/// `detail` is diagnostic-only and must be redacted before it is populated.
#[derive(Debug, Clone, Serialize, Error)]
#[error("{message}")]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub code: ErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    pub retryable: bool,
    pub correlation_id: String,
}

impl AppError {
    pub fn new(code: ErrorCode) -> Self {
        Self {
            code,
            message: code.default_message().to_owned(),
            detail: None,
            retryable: code.retryable(),
            correlation_id: next_correlation_id(),
        }
    }

    #[must_use]
    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }
}

fn next_correlation_id() -> String {
    let seq = CORRELATION_SEQ.fetch_add(1, Ordering::Relaxed);
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis())
        .unwrap_or(0);
    format!("{millis:x}-{seq:04x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retryable_matches_the_documented_taxonomy() {
        assert!(ErrorCode::AccessDenied.retryable());
        assert!(ErrorCode::Timeout.retryable());
        assert!(ErrorCode::Busy.retryable());
        assert!(!ErrorCode::InvalidInput.retryable());
        assert!(!ErrorCode::NotPermitted.retryable());
        assert!(!ErrorCode::Cancelled.retryable());
    }

    #[test]
    fn every_code_has_a_non_empty_message() {
        let codes = [
            ErrorCode::InvalidInput,
            ErrorCode::NotPermitted,
            ErrorCode::AccessDenied,
            ErrorCode::NotFound,
            ErrorCode::Unsupported,
            ErrorCode::Timeout,
            ErrorCode::Busy,
            ErrorCode::Cancelled,
            ErrorCode::DiskFull,
            ErrorCode::DbError,
            ErrorCode::Internal,
        ];

        for code in codes {
            assert!(!code.default_message().is_empty());
        }
    }

    #[test]
    fn correlation_ids_are_unique() {
        let first = AppError::new(ErrorCode::Internal).correlation_id;
        let second = AppError::new(ErrorCode::Internal).correlation_id;
        assert_ne!(first, second);
    }
}
