//! `scan_sessions` repository: lifecycle and counters only.
//!
//! Paths are never written. `roots_json` is left at its `'[]'` default until the
//! retention decision in `docs/plan.md` §12 allows storing scope.

use rusqlite::params;

use crate::adapters::persistence::Database;
use crate::ports::sessions::{ScanSessionStore, SessionFinish, SessionStart, SessionStoreError};

/// Module name stored for scans of the filesystem.
const MODULE_STORAGE: &str = "storage";

/// Sessions that were still open when the previous run ended.
const OPEN_STATES: &str = "('queued', 'running', 'cancelling')";

impl ScanSessionStore for Database {
    fn begin(&self, start: &SessionStart) -> Result<(), SessionStoreError> {
        let connection = self.connection().map_err(log_store_error)?;
        connection
            .execute(
                "INSERT INTO scan_sessions
                   (id, module, options_json, status, started_at, engine_version, policy_version)
                 VALUES (?1, ?2, ?3, 'running', ?4, ?5, ?6)",
                params![
                    start.id,
                    MODULE_STORAGE,
                    start.options_json,
                    start.started_at_ms,
                    start.engine_version,
                    start.policy_version,
                ],
            )
            .map(|_| ())
            .map_err(log_sql_error)
    }

    fn finish(&self, id: &str, finish: &SessionFinish) -> Result<(), SessionStoreError> {
        let counters = &finish.counters;
        let connection = self.connection().map_err(log_store_error)?;
        let changed = connection
            .execute(
                &format!(
                    "UPDATE scan_sessions SET
                       status = ?2, finished_at = ?3, completeness = ?4,
                       entries_seen = ?5, files_seen = ?6, directories_seen = ?7,
                       reparse_points_seen = ?8, bytes_seen = ?9, files_skipped = ?10,
                       files_denied = ?11, files_disappeared = ?12,
                       files_unsupported = ?13, files_failed = ?14,
                       cancel_reason = ?15, error_code = ?16
                     WHERE id = ?1 AND status IN {OPEN_STATES}"
                ),
                params![
                    id,
                    finish.status,
                    finish.finished_at_ms,
                    finish.completeness,
                    to_i64(counters.entries),
                    to_i64(counters.files),
                    to_i64(counters.directories),
                    to_i64(counters.reparse_points),
                    to_i64(counters.bytes),
                    to_i64(counters.skipped),
                    to_i64(counters.denied),
                    to_i64(counters.disappeared),
                    to_i64(counters.unsupported),
                    to_i64(counters.failed),
                    finish.cancel_reason,
                    finish.error_code,
                ],
            )
            .map_err(log_sql_error)?;

        if changed == 1 {
            Ok(())
        } else {
            // Unknown ID (the begin write failed) or an already-ended session.
            tracing::warn!(changed, "scan session was not updated");
            Err(SessionStoreError)
        }
    }

    fn mark_interrupted(&self, now_ms: i64) -> Result<usize, SessionStoreError> {
        let connection = self.connection().map_err(log_store_error)?;
        connection
            .execute(
                &format!(
                    "UPDATE scan_sessions SET status = 'interrupted', finished_at = ?1
                     WHERE status IN {OPEN_STATES}"
                ),
                params![now_ms],
            )
            .map_err(log_sql_error)
    }
}

/// SQLite integers are signed 64-bit; clamp instead of wrapping.
fn to_i64(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

fn log_store_error(error: crate::adapters::persistence::StoreError) -> SessionStoreError {
    tracing::warn!(%error, "scan session store unavailable");
    SessionStoreError
}

fn log_sql_error(error: rusqlite::Error) -> SessionStoreError {
    tracing::warn!(%error, "scan session write failed");
    SessionStoreError
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::traversal::TraversalSummary;

    fn start(id: &str) -> SessionStart {
        SessionStart {
            id: id.to_owned(),
            started_at_ms: 1_000,
            options_json: r#"{"maxEntries":10}"#.to_owned(),
            engine_version: "0.1.0",
            policy_version: "traversal-1",
        }
    }

    fn finish(status: &'static str) -> SessionFinish {
        SessionFinish {
            status,
            finished_at_ms: 2_000,
            completeness: Some("partial"),
            counters: TraversalSummary {
                entries: 9,
                files: 5,
                directories: 3,
                reparse_points: 1,
                bytes: 123,
                skipped: 1,
                denied: 1,
                disappeared: 1,
                unsupported: 1,
                failed: 1,
                ..TraversalSummary::default()
            },
            cancel_reason: None,
            error_code: None,
        }
    }

    type Row = (String, Option<i64>, Option<String>, i64, i64, i64, String);

    fn row(database: &Database, id: &str) -> Row {
        database
            .connection()
            .unwrap()
            .query_row(
                "SELECT status, finished_at, completeness, entries_seen, bytes_seen,
                        files_denied, roots_json
                 FROM scan_sessions WHERE id = ?1",
                [id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                    ))
                },
            )
            .unwrap()
    }

    #[test]
    fn a_session_is_recorded_running_then_finished_with_counters() {
        let database = Database::open_in_memory().expect("in-memory database");
        database.begin(&start("s1")).unwrap();

        let running = row(&database, "s1");
        assert_eq!(running.0, "running");
        assert_eq!(running.1, None);
        assert_eq!(running.6, "[]", "no root path is stored");

        database.finish("s1", &finish("completed")).unwrap();
        let done = row(&database, "s1");
        assert_eq!(done.0, "completed");
        assert_eq!(done.1, Some(2_000));
        assert_eq!(done.2.as_deref(), Some("partial"));
        assert_eq!(done.3, 9);
        assert_eq!(done.4, 123);
        assert_eq!(done.5, 1);
    }

    #[test]
    fn an_ended_session_is_never_overwritten() {
        let database = Database::open_in_memory().expect("in-memory database");
        database.begin(&start("s1")).unwrap();
        database.finish("s1", &finish("cancelled")).unwrap();

        assert_eq!(
            database.finish("s1", &finish("completed")),
            Err(SessionStoreError)
        );
        assert_eq!(row(&database, "s1").0, "cancelled");
    }

    #[test]
    fn finishing_an_unknown_session_is_an_error_not_a_panic() {
        let database = Database::open_in_memory().expect("in-memory database");
        assert_eq!(
            database.finish("missing", &finish("completed")),
            Err(SessionStoreError)
        );
    }

    #[test]
    fn a_duplicate_id_is_refused() {
        let database = Database::open_in_memory().expect("in-memory database");
        database.begin(&start("s1")).unwrap();
        assert_eq!(database.begin(&start("s1")), Err(SessionStoreError));
    }

    #[test]
    fn open_sessions_become_interrupted_and_ended_ones_do_not() {
        let database = Database::open_in_memory().expect("in-memory database");
        database.begin(&start("open")).unwrap();
        database.begin(&start("done")).unwrap();
        database.finish("done", &finish("completed")).unwrap();

        assert_eq!(database.mark_interrupted(3_000).unwrap(), 1);
        assert_eq!(database.mark_interrupted(3_000).unwrap(), 0);

        let interrupted = row(&database, "open");
        assert_eq!(interrupted.0, "interrupted");
        assert_eq!(interrupted.1, Some(3_000));
        assert_eq!(row(&database, "done").0, "completed");
    }

    #[test]
    fn counters_beyond_i64_are_clamped_not_wrapped() {
        assert_eq!(to_i64(u64::MAX), i64::MAX);
        assert_eq!(to_i64(5), 5);
    }
}
