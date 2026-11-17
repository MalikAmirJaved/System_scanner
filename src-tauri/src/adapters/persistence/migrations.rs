//! Hand-written, forward-only schema migrations.
//!
//! Migrations are embedded at compile time, so a release can never apply a file
//! that was not part of its build. Each migration is immutable once released:
//! corrections are new migrations. The applied version **and** checksum live in
//! the `schema_migrations` table, so editing a released file is detected at
//! startup instead of silently skipping the change.

/// Highest schema version this build understands.
pub const SCHEMA_VERSION: u32 = 2;

/// One immutable schema change.
pub struct Migration {
    pub version: u32,
    pub name: &'static str,
    pub checksum: i64,
    pub sql: &'static str,
}

/// Ordered migration list. Versions must be unique and strictly increasing.
pub const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "initial_schema",
        checksum: checksum(include_str!("../../../migrations/0001_initial.sql")),
        sql: include_str!("../../../migrations/0001_initial.sql"),
    },
    Migration {
        version: 2,
        name: "scan_session_results",
        checksum: checksum(include_str!(
            "../../../migrations/0002_scan_session_results.sql"
        )),
        sql: include_str!("../../../migrations/0002_scan_session_results.sql"),
    },
];

/// FNV-1a over the migration text.
///
/// This is a drift detector, not a security control: it catches an edited
/// released migration, not a deliberate forgery. A migration that must change
/// is a new migration.
pub const fn checksum(sql: &str) -> i64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let bytes = sql.as_bytes();
    let mut index = 0;

    while index < bytes.len() {
        hash ^= bytes[index] as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        index += 1;
    }

    hash as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checksum_is_stable_and_content_sensitive() {
        assert_eq!(checksum("abc"), 0xe71f_a219_0541_574b_u64 as i64);
        assert_ne!(checksum("abc"), checksum("abd"));
    }

    #[test]
    fn versions_are_strictly_increasing_and_start_at_one() {
        let versions: Vec<u32> = MIGRATIONS.iter().map(|item| item.version).collect();

        assert_eq!(versions.first().copied(), Some(1));
        assert!(
            versions.windows(2).all(|pair| pair[0] < pair[1]),
            "versions must be strictly increasing: {versions:?}"
        );
        assert_eq!(
            versions.last().copied(),
            Some(SCHEMA_VERSION),
            "SCHEMA_VERSION must match the last migration"
        );
    }

    #[test]
    fn every_migration_has_non_empty_sql_and_a_matching_checksum() {
        for migration in MIGRATIONS {
            assert!(!migration.sql.trim().is_empty(), "{}", migration.name);
            assert_eq!(
                migration.checksum,
                checksum(migration.sql),
                "checksum constant for {} is stale",
                migration.name
            );
        }
    }
}
