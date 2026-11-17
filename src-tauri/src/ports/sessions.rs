//! Port for recording scan session lifecycle and result counters.
//!
//! Counters and status only. No path, file name, or per-entry data crosses this
//! port: the retention decision in `docs/plan.md` §12 is still open, so storing
//! file listings is out of scope.

use crate::domain::traversal::TraversalSummary;

/// The opaque failure of a store call. Details are logged by the adapter; the
/// caller only needs to know the write did not happen.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("the scan session could not be recorded")]
pub struct SessionStoreError;

/// A session that has just started.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionStart {
    pub id: String,
    pub started_at_ms: i64,
    /// Limits and budgets as JSON. Must not contain paths.
    pub options_json: String,
    pub engine_version: &'static str,
    pub policy_version: &'static str,
}

/// How a session ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionFinish {
    /// `completed`, `failed`, or `cancelled`.
    pub status: &'static str,
    pub finished_at_ms: i64,
    /// `complete`, `partial`, `truncated`, or `cancelled`; `None` when the walk
    /// produced no result (a failed job).
    pub completeness: Option<&'static str>,
    pub counters: TraversalSummary,
    pub cancel_reason: Option<&'static str>,
    pub error_code: Option<&'static str>,
}

pub trait ScanSessionStore: Send + Sync {
    /// Record a session as `running`.
    fn begin(&self, start: &SessionStart) -> Result<(), SessionStoreError>;

    /// Record the end of a session. A session that already ended is never
    /// overwritten; that returns an error.
    fn finish(&self, id: &str, finish: &SessionFinish) -> Result<(), SessionStoreError>;

    /// Mark sessions left `queued`, `running`, or `cancelling` by a previous run
    /// as `interrupted`. Returns how many were changed.
    fn mark_interrupted(&self, now_ms: i64) -> Result<usize, SessionStoreError>;
}
