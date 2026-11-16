//! SQLite access layer.
//!
//! Rules:
//! - One blocking connection behind a mutex. A local single-user app has one
//!   writer; WAL lets future readers run alongside it.
//! - Every call is synchronous. Commands must run database work on
//!   `tauri::async_runtime::spawn_blocking`, never on the UI thread.
//! - Opening the database configures the connection, applies pending migrations,
//!   and refuses a file written by a newer build. A failure leaves the file
//!   unchanged (each migration is one transaction).

use std::fs;
use std::path::Path;
use std::sync::{Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::Connection;
use thiserror::Error;

use crate::adapters::persistence::migrations::{MIGRATIONS, SCHEMA_VERSION};

/// File name inside the Tauri app data directory.
pub const DATABASE_FILE_NAME: &str = "system-scanner.db";

/// Connection settings from [`crate::adapters::persistence`] and `DATABASE.md`.
///
/// `journal_mode` falls back to `memory` for an in-memory database, which SQLite
/// reports instead of failing.
const PRAGMAS: &str = "\
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA foreign_keys = ON;
PRAGMA busy_timeout = 5000;";

const BOOKKEEPING_TABLE: &str = "\
CREATE TABLE IF NOT EXISTS schema_migrations (
  version       INTEGER PRIMARY KEY,
  name          TEXT NOT NULL,
  checksum      INTEGER NOT NULL,
  applied_at_ms INTEGER NOT NULL
);";

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("the database directory could not be created: {0}")]
    Directory(#[source] std::io::Error),
    #[error("the database could not be opened: {0}")]
    Open(#[source] rusqlite::Error),
    #[error("the schema bookkeeping table could not be read or written: {0}")]
    Bookkeeping(#[source] rusqlite::Error),
    #[error("the database schema is version {found}, but this build supports {supported}")]
    NewerSchema { found: u32, supported: u32 },
    #[error("migration {version} changed after it was applied")]
    ChecksumMismatch { version: u32 },
    #[error("migration {version} ({name}) failed: {source}")]
    Migration {
        version: u32,
        name: &'static str,
        #[source]
        source: rusqlite::Error,
    },
    #[error("the database connection is unusable because a previous holder panicked")]
    Poisoned,
}

/// Owned, migrated database handle.
///
/// The connection stays open for the process lifetime. Repositories will take
/// the guard; for now only the startup path and tests use it.
pub struct Database {
    connection: Mutex<Connection>,
}

impl Database {
    /// Opens (or creates) a database file and applies pending migrations.
    ///
    /// The parent directory is created first, so a fresh profile needs no
    /// separate setup step.
    pub fn open(path: &Path) -> Result<Self, StoreError> {
        if let Some(directory) = path.parent() {
            fs::create_dir_all(directory).map_err(StoreError::Directory)?;
        }

        let connection = Connection::open(path).map_err(StoreError::Open)?;
        Self::from_connection(connection)
    }

    /// Version recorded in `schema_migrations`, or 0 before the first migration.
    pub fn schema_version(&self) -> Result<u32, StoreError> {
        self.connection()?
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
                [],
                |row| row.get::<_, u32>(0),
            )
            .map_err(StoreError::Bookkeeping)
    }

    #[cfg(test)]
    fn open_in_memory() -> Result<Self, StoreError> {
        let connection = Connection::open_in_memory().map_err(StoreError::Open)?;
        Self::from_connection(connection)
    }

    fn from_connection(connection: Connection) -> Result<Self, StoreError> {
        connection
            .execute_batch(PRAGMAS)
            .map_err(StoreError::Bookkeeping)?;

        let database = Self {
            connection: Mutex::new(connection),
        };
        database.migrate()?;
        Ok(database)
    }

    fn connection(&self) -> Result<MutexGuard<'_, Connection>, StoreError> {
        self.connection.lock().map_err(|_| StoreError::Poisoned)
    }

    fn migrate(&self) -> Result<(), StoreError> {
        let mut connection = self.connection()?;

        connection
            .execute_batch(BOOKKEEPING_TABLE)
            .map_err(StoreError::Bookkeeping)?;

        let applied = {
            let mut statement = connection
                .prepare("SELECT version, checksum FROM schema_migrations ORDER BY version")
                .map_err(StoreError::Bookkeeping)?;
            let rows = statement
                .query_map([], |row| Ok((row.get::<_, u32>(0)?, row.get::<_, i64>(1)?)))
                .map_err(StoreError::Bookkeeping)?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(StoreError::Bookkeeping)?
        };

        let current = applied.last().map_or(0, |(version, _)| *version);
        if current > SCHEMA_VERSION {
            return Err(StoreError::NewerSchema {
                found: current,
                supported: SCHEMA_VERSION,
            });
        }

        for (version, stored) in &applied {
            let known = MIGRATIONS.iter().find(|item| item.version == *version);
            if known.is_some_and(|item| item.checksum != *stored) {
                return Err(StoreError::ChecksumMismatch { version: *version });
            }
        }

        for migration in MIGRATIONS.iter().filter(|item| item.version > current) {
            let transaction = connection
                .transaction()
                .map_err(|source| StoreError::Migration {
                    version: migration.version,
                    name: migration.name,
                    source,
                })?;

            transaction
                .execute_batch(migration.sql)
                .and_then(|()| {
                    transaction.execute(
                        "INSERT INTO schema_migrations (version, name, checksum, applied_at_ms)
                         VALUES (?1, ?2, ?3, ?4)",
                        rusqlite::params![
                            migration.version,
                            migration.name,
                            migration.checksum,
                            now_ms()
                        ],
                    )
                })
                .and_then(|_| transaction.commit())
                .map_err(|source| StoreError::Migration {
                    version: migration.version,
                    name: migration.name,
                    source,
                })?;
        }

        Ok(())
    }
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|elapsed| i64::try_from(elapsed.as_millis()).ok())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(label: &str) -> Self {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|elapsed| elapsed.as_nanos())
                .unwrap_or(0);
            let path = std::env::temp_dir().join(format!(
                "system-scanner-db-{label}-{}-{nanos}",
                std::process::id()
            ));
            fs::create_dir_all(&path).expect("temp dir must be creatable");
            Self(path)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn a_fresh_database_reaches_the_latest_schema() {
        let database = Database::open_in_memory().expect("in-memory database must open");

        assert_eq!(database.schema_version().unwrap(), SCHEMA_VERSION);

        let connection = database.connection().unwrap();
        let columns: Vec<String> = {
            let mut statement = connection
                .prepare("SELECT name FROM pragma_table_info('scan_sessions')")
                .unwrap();
            statement
                .query_map([], |row| row.get(0))
                .unwrap()
                .collect::<Result<_, _>>()
                .unwrap()
        };

        for expected in ["id", "module", "roots_json", "status", "started_at"] {
            assert!(
                columns.iter().any(|column| column == expected),
                "scan_sessions is missing {expected}: {columns:?}"
            );
        }
        assert_eq!(
            connection
                .query_row("PRAGMA foreign_keys", [], |row| row.get::<_, i64>(0))
                .unwrap(),
            1
        );
    }

    #[test]
    fn a_file_database_uses_wal_and_creates_its_directory() {
        let directory = TempDir::new("wal");
        let path = directory.0.join("nested").join(DATABASE_FILE_NAME);

        let database = Database::open(&path).expect("file database must open");

        let mode: String = database
            .connection()
            .unwrap()
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .unwrap();
        assert_eq!(mode.to_ascii_lowercase(), "wal");
        assert!(path.exists());
    }

    #[test]
    fn reopening_applies_nothing_new() {
        let directory = TempDir::new("reopen");
        let path = directory.0.join(DATABASE_FILE_NAME);

        drop(Database::open(&path).expect("first open must succeed"));
        let database = Database::open(&path).expect("second open must succeed");

        assert_eq!(database.schema_version().unwrap(), SCHEMA_VERSION);
        let applied: i64 = database
            .connection()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(applied, i64::from(SCHEMA_VERSION));
    }

    #[test]
    fn a_newer_schema_is_refused_without_changes() {
        let connection = Connection::open_in_memory().unwrap();
        connection.execute_batch(BOOKKEEPING_TABLE).unwrap();
        connection
            .execute(
                "INSERT INTO schema_migrations (version, name, checksum, applied_at_ms)
                 VALUES (?1, 'future', 0, 0)",
                rusqlite::params![SCHEMA_VERSION + 1],
            )
            .unwrap();

        let error = Database::from_connection(connection)
            .err()
            .expect("must refuse");

        assert!(matches!(error, StoreError::NewerSchema { .. }), "{error:?}");
        assert_eq!(
            error.to_string(),
            format!(
                "the database schema is version {}, but this build supports {SCHEMA_VERSION}",
                SCHEMA_VERSION + 1
            )
        );
    }

    #[test]
    fn a_changed_released_migration_is_detected() {
        let connection = Connection::open_in_memory().unwrap();
        connection.execute_batch(BOOKKEEPING_TABLE).unwrap();
        connection
            .execute(
                "INSERT INTO schema_migrations (version, name, checksum, applied_at_ms)
                 VALUES (1, 'initial_schema', 0, 0)",
                [],
            )
            .unwrap();

        let error = Database::from_connection(connection)
            .err()
            .expect("must refuse");

        assert!(
            matches!(error, StoreError::ChecksumMismatch { version: 1 }),
            "{error:?}"
        );
    }
}
