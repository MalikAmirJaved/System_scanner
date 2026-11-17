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
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, OpenFlags};
use thiserror::Error;

use crate::adapters::persistence::migrations::{MIGRATIONS, SCHEMA_VERSION};

/// File name inside the Tauri app data directory.
pub const DATABASE_FILE_NAME: &str = "system-scanner.db";

/// Subdirectory, beside the database file, that holds pre-migration snapshots.
pub const BACKUP_DIRECTORY_NAME: &str = "backups";

/// Snapshots kept on disk. `docs/BACKUP_POLICY.md` §3 requires the newest five.
const MAX_BACKUPS: usize = 5;

/// File-name prefix for a snapshot; the schema version and timestamp follow.
const BACKUP_PREFIX: &str = "system-scanner-v";

/// Suffix of a snapshot file.
const BACKUP_SUFFIX: &str = ".db";

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
    #[error("the backup directory could not be created: {0}")]
    BackupDirectory(#[source] std::io::Error),
    #[error("the pre-migration backup could not be written: {0}")]
    Backup(#[source] rusqlite::Error),
    #[error("the pre-migration backup could not be verified: {0}")]
    BackupVerification(#[source] rusqlite::Error),
    #[error("the pre-migration backup failed its integrity check: {0}")]
    BackupUnhealthy(String),
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
        let database = Self {
            connection: Mutex::new(connection),
        };

        database.configure()?;
        database.backup_before_migration(path);
        database.migrate()?;
        Ok(database)
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
    pub(super) fn open_in_memory() -> Result<Self, StoreError> {
        let connection = Connection::open_in_memory().map_err(StoreError::Open)?;
        Self::from_connection(connection)
    }

    #[cfg(test)]
    fn from_connection(connection: Connection) -> Result<Self, StoreError> {
        let database = Self {
            connection: Mutex::new(connection),
        };

        database.configure()?;
        database.migrate()?;
        Ok(database)
    }

    fn configure(&self) -> Result<(), StoreError> {
        self.connection()?
            .execute_batch(PRAGMAS)
            .map_err(StoreError::Bookkeeping)
    }

    /// Takes one consistent snapshot if the file needs a migration.
    ///
    /// Deliberately best-effort, like the rest of persistence: a full disk or a
    /// denied directory must not stop the app from starting. Every released
    /// migration is additive, so a missing snapshot cannot lose history today;
    /// a non-additive migration must make this mandatory first (see
    /// `docs/BACKUP_POLICY.md` §3 and KI-014).
    fn backup_before_migration(&self, database_path: &Path) {
        let Some(directory) = database_path.parent() else {
            return;
        };

        let current = {
            let Ok(connection) = self.connection() else {
                tracing::warn!("database backup skipped: the connection is poisoned");
                return;
            };

            match stored_version(&connection) {
                // No applied migrations means there is no history to protect.
                Ok(None) => return,
                Ok(Some(version)) => version,
                Err(error) => {
                    tracing::warn!(%error, "database backup skipped: the schema version is unreadable");
                    return;
                }
            }
        };

        if current >= SCHEMA_VERSION {
            return;
        }

        let backups = directory.join(BACKUP_DIRECTORY_NAME);
        let snapshot = self
            .connection()
            .and_then(|connection| write_snapshot(&connection, &backups, current, now_ms()));

        match snapshot {
            Ok(path) => {
                tracing::info!(
                    backup = %path.display(),
                    from_version = current,
                    "database backed up before migration"
                );
                prune_backups(&backups, MAX_BACKUPS);
            }
            Err(error) => {
                tracing::warn!(%error, "database backup before migration failed; continuing");
            }
        }
    }

    pub(super) fn connection(&self) -> Result<MutexGuard<'_, Connection>, StoreError> {
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

/// Applied schema version, or `None` when no migration has been recorded.
fn stored_version(connection: &Connection) -> rusqlite::Result<Option<u32>> {
    let exists: bool = connection.query_row(
        "SELECT EXISTS(
           SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'schema_migrations'
         )",
        [],
        |row| row.get(0),
    )?;

    if !exists {
        return Ok(None);
    }

    connection.query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
        row.get(0)
    })
}

/// Writes and verifies one snapshot, returning its path.
///
/// `VACUUM INTO` is the only supported copy: it produces a single consistent
/// file without the WAL sidecars, and it must not be replaced by a file copy.
fn write_snapshot(
    connection: &Connection,
    directory: &Path,
    version: u32,
    timestamp: i64,
) -> Result<PathBuf, StoreError> {
    fs::create_dir_all(directory).map_err(StoreError::BackupDirectory)?;

    let path = unique_backup_path(directory, version, timestamp);
    // `VACUUM INTO` takes a text path; app-data paths are UTF-8 in practice,
    // and lossy conversion can only ever name a different file, never overwrite
    // one, because the destination must not exist.
    let target = path.to_string_lossy().into_owned();
    connection
        .execute("VACUUM INTO ?1", [target.as_str()])
        .map_err(StoreError::Backup)?;

    if let Err(error) = verify_snapshot(&path) {
        let _ = fs::remove_file(&path);
        return Err(error);
    }

    Ok(path)
}

/// Opens the snapshot read-only and runs `PRAGMA quick_check`.
fn verify_snapshot(path: &Path) -> Result<(), StoreError> {
    let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(StoreError::BackupVerification)?;
    let status: String = connection
        .query_row("PRAGMA quick_check", [], |row| row.get(0))
        .map_err(StoreError::BackupVerification)?;

    if status != "ok" {
        return Err(StoreError::BackupUnhealthy(status));
    }

    Ok(())
}

fn unique_backup_path(directory: &Path, version: u32, timestamp: i64) -> PathBuf {
    let stem = format!("{BACKUP_PREFIX}{version:03}-{timestamp}");
    let mut candidate = directory.join(format!("{stem}{BACKUP_SUFFIX}"));
    let mut suffix = 1;

    while candidate.exists() {
        candidate = directory.join(format!("{stem}-{suffix}{BACKUP_SUFFIX}"));
        suffix += 1;
    }

    candidate
}

/// Deletes the oldest snapshots beyond `keep`, leaving other files alone.
///
/// Names embed a zero-padded version and a millisecond timestamp, so sorting by
/// name is sorting by age.
fn prune_backups(directory: &Path, keep: usize) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };

    let mut backups: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| is_backup_file(path))
        .collect();

    backups.sort();
    let excess = backups.len().saturating_sub(keep);

    for path in backups.into_iter().take(excess) {
        if let Err(error) = fs::remove_file(&path) {
            tracing::warn!(%error, backup = %path.display(), "old database backup could not be removed");
        }
    }
}

fn is_backup_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with(BACKUP_PREFIX) && name.ends_with(BACKUP_SUFFIX))
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
    fn a_v1_database_upgrades_to_v2_without_losing_rows() {
        use crate::adapters::persistence::migrations::checksum;

        let connection = Connection::open_in_memory().unwrap();
        connection.execute_batch(BOOKKEEPING_TABLE).unwrap();
        let first = &MIGRATIONS[0];
        connection.execute_batch(first.sql).unwrap();
        connection
            .execute(
                "INSERT INTO schema_migrations (version, name, checksum, applied_at_ms)
                 VALUES (1, ?1, ?2, 0)",
                rusqlite::params![first.name, checksum(first.sql)],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO scan_sessions
                   (id, module, status, started_at, engine_version, policy_version, files_seen)
                 VALUES ('old', 'storage', 'completed', 1, '0.1.0', 'p1', 7)",
                [],
            )
            .unwrap();

        let database = Database::from_connection(connection).expect("upgrade must succeed");

        assert_eq!(database.schema_version().unwrap(), 2);
        let (files, entries, completeness): (i64, i64, Option<String>) = database
            .connection()
            .unwrap()
            .query_row(
                "SELECT files_seen, entries_seen, completeness FROM scan_sessions WHERE id = 'old'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(files, 7, "existing counters survive");
        assert_eq!(entries, 0, "new counters default to zero");
        assert_eq!(completeness, None, "an old row has no verdict");
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

    /// Writes a schema-v1 database file with one completed session row.
    fn create_v1_file_database(path: &Path) {
        use crate::adapters::persistence::migrations::checksum;

        let connection = Connection::open(path).unwrap();
        connection.execute_batch(BOOKKEEPING_TABLE).unwrap();
        let first = &MIGRATIONS[0];
        connection.execute_batch(first.sql).unwrap();
        connection
            .execute(
                "INSERT INTO schema_migrations (version, name, checksum, applied_at_ms)
                 VALUES (1, ?1, ?2, 0)",
                rusqlite::params![first.name, checksum(first.sql)],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO scan_sessions
                   (id, module, status, started_at, engine_version, policy_version, files_seen)
                 VALUES ('old', 'storage', 'completed', 1, '0.1.0', 'p1', 7)",
                [],
            )
            .unwrap();
    }

    fn backup_files(directory: &Path) -> Vec<PathBuf> {
        let mut files: Vec<PathBuf> = fs::read_dir(directory)
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .map(|entry| entry.path())
                    .filter(|path| is_backup_file(path))
                    .collect()
            })
            .unwrap_or_default();
        files.sort();
        files
    }

    #[test]
    fn a_pending_migration_snapshots_the_database_and_the_snapshot_restores() {
        let directory = TempDir::new("backup");
        let path = directory.0.join(DATABASE_FILE_NAME);
        create_v1_file_database(&path);

        let database = Database::open(&path).expect("migration must succeed");
        assert_eq!(database.schema_version().unwrap(), SCHEMA_VERSION);
        drop(database);

        let backups = directory.0.join(BACKUP_DIRECTORY_NAME);
        let snapshots = backup_files(&backups);
        assert_eq!(snapshots.len(), 1, "{snapshots:?}");

        // The snapshot is the pre-migration copy: schema v1 with the row intact,
        // and it passes the documented integrity check.
        {
            let snapshot =
                Connection::open_with_flags(&snapshots[0], OpenFlags::SQLITE_OPEN_READ_ONLY)
                    .unwrap();
            let version: u32 = snapshot
                .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
                    row.get(0)
                })
                .unwrap();
            let files: i64 = snapshot
                .query_row(
                    "SELECT files_seen FROM scan_sessions WHERE id = 'old'",
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            let health: String = snapshot
                .query_row("PRAGMA quick_check", [], |row| row.get(0))
                .unwrap();

            assert_eq!(version, 1);
            assert_eq!(files, 7);
            assert_eq!(health, "ok");
        }

        // Restore drill: the snapshot is a usable database and upgrades in place.
        let restored = Database::open(&snapshots[0]).expect("snapshot must restore");
        assert_eq!(restored.schema_version().unwrap(), SCHEMA_VERSION);
        let files: i64 = restored
            .connection()
            .unwrap()
            .query_row(
                "SELECT files_seen FROM scan_sessions WHERE id = 'old'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(files, 7, "history survives a restore");
    }

    #[test]
    fn an_up_to_date_database_is_not_snapshotted() {
        let directory = TempDir::new("no-backup");
        let path = directory.0.join(DATABASE_FILE_NAME);

        drop(Database::open(&path).expect("first open must succeed"));
        drop(Database::open(&path).expect("second open must succeed"));

        assert!(!directory.0.join(BACKUP_DIRECTORY_NAME).exists());
    }

    #[test]
    fn a_failed_backup_does_not_block_the_migration() {
        let directory = TempDir::new("backup-fail");
        let path = directory.0.join(DATABASE_FILE_NAME);
        create_v1_file_database(&path);

        // A file where the backup directory belongs makes every write fail.
        fs::write(directory.0.join(BACKUP_DIRECTORY_NAME), b"not a directory").unwrap();

        let database = Database::open(&path).expect("migration must still run");
        assert_eq!(database.schema_version().unwrap(), SCHEMA_VERSION);
        assert!(backup_files(&directory.0).is_empty());
    }

    #[test]
    fn old_snapshots_are_pruned_to_the_retention_limit() {
        let directory = TempDir::new("prune");
        fs::create_dir_all(&directory.0).unwrap();

        for index in 0..(MAX_BACKUPS + 3) {
            fs::write(
                directory
                    .0
                    .join(format!("{BACKUP_PREFIX}001-{index:04}0{BACKUP_SUFFIX}")),
                b"snapshot",
            )
            .unwrap();
        }
        fs::write(directory.0.join("unrelated.db"), b"keep me").unwrap();

        prune_backups(&directory.0, MAX_BACKUPS);

        let names: Vec<String> = fs::read_dir(&directory.0)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();

        assert_eq!(names.len(), MAX_BACKUPS + 1, "{names:?}");
        assert!(names.iter().any(|name| name == "unrelated.db"));
        assert!(!names.iter().any(|name| name.contains("-00000.db")));
        assert!(names.iter().any(|name| name.contains("-00070.db")));
    }

    #[test]
    fn a_corrupt_snapshot_fails_verification() {
        let directory = TempDir::new("verify");
        let path = directory.0.join("broken.db");
        fs::write(&path, b"this is not a database").unwrap();

        let error = verify_snapshot(&path).expect_err("must be rejected");
        assert!(
            matches!(
                error,
                StoreError::BackupVerification(_) | StoreError::BackupUnhealthy(_)
            ),
            "{error:?}"
        );
    }
}
