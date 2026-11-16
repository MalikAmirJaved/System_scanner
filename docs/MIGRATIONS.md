# Migrations

> **Status:** **Implemented for the SQLite schema** — hand-written, forward-only,
> embedded migrations with a `schema_migrations` bookkeeping table and FNV-1a
> checksums. Settings, export, and quarantine-manifest migration remain
> **proposed**.
> **Last updated:** 2026-11-16

---

## 1. Scope

Migrations apply to:

| Artifact | Versioned by | Status |
|---|---|---|
| SQLite schema | `schema_migrations` table (version + checksum) | ✅ Implemented |
| Settings file | `schema_version` field | ⏳ Proposed |
| Exported reports | `schema_version` in the export envelope | ⏳ Proposed |
| Quarantine manifest | `manifest_version` | ⏳ Proposed |
| App data directory layout | app version + documented note | ⏳ Proposed |

## 2. Principles

1. **Forward-only.** No down-migrations in production. Rollback = restore backup
   + reinstall the previous app version.
2. **Atomic.** Each migration runs in a transaction; failure leaves the DB at the
   previous version.
3. **Tested from every released version**, not only from scratch.
4. **Never destructive by default.** Dropping a column/table that holds user data
   requires explicit approval and a backup step.
5. **Versioned in code.** Migration files are immutable once released; corrections
   are new migrations.
6. **Offline.** No migration may require the network.

## 3. Layout

```text
src-tauri/migrations/
  0001_initial.sql
```

Rules:

- `NNNN_snake_case_description.sql`, zero-padded, strictly increasing.
- One logical change per migration.
- No `IF NOT EXISTS` masking of unexpected state in a new migration.
- Files are embedded at compile time with `include_str!`, so a release can never
  apply a migration file that was not part of its build.
- Applied migrations are checksummed (FNV-1a); a changed checksum on a released
  migration refuses to open the database.
- The checksum is a **drift detector, not a security control**. It catches an
  accidental edit, not a deliberate forgery.

`src-tauri/src/adapters/persistence/migrations.rs` holds the ordered `MIGRATIONS`
list and `SCHEMA_VERSION`. `src-tauri/src/adapters/persistence/sqlite.rs` applies
them.

## 4. Runner behavior

Implemented (`Database::open`):

```text
configure pragmas (WAL, synchronous=NORMAL, foreign_keys=ON, busy_timeout=5000)
create schema_migrations if missing
read applied (version, checksum) rows
if max(version) > SCHEMA_VERSION        -> refuse (NewerSchema), file unchanged
for each applied row whose version is known:
    if stored checksum != code checksum -> refuse (ChecksumMismatch), file unchanged
for each migration with version > max(applied):
    begin transaction
      execute migration SQL
      insert (version, name, checksum, applied_at_ms) into schema_migrations
    commit
on error: rollback, keep DB at previous version, return the failing version + name
```

Additional rules:

- A **pre-migration backup (`VACUUM INTO`) before a non-additive migration is not
  implemented.** v1 is purely additive, so nothing can be lost today. It becomes
  required before the first migration that can drop or rewrite data (§11).
- Show progress for long migrations; do not block the UI without feedback
  (not started — no long migration exists yet).
- Never run migrations concurrently with an active scan: the single
  `Mutex<Connection>` serializes access.
- `PRAGMA quick_check` after migration is **not implemented**.

## 5. Test matrix

| Starting state | Expected | Covered |
|---|---|---|
| Empty / new install | Fresh schema at latest version, `journal_mode=wal` for a file DB | ✅ `a_fresh_database_reaches_the_latest_schema`, `a_file_database_uses_wal_and_creates_its_directory` |
| Reopen an up-to-date DB | Applies nothing new; `schema_migrations` count is stable | ✅ `reopening_applies_nothing_new` |
| DB from a newer app version | Refuses, explains, does not downgrade | ✅ `a_newer_schema_is_refused_without_changes` |
| Released migration edited | Refuses on checksum mismatch | ✅ `a_changed_released_migration_is_detected` |
| Migration list invariants | Versions start at 1, strictly increase, end at `SCHEMA_VERSION`; checksums match SQL | ✅ `migrations.rs` tests |
| Each released version | Upgrades to latest, data intact | ⏳ Not applicable — only v1 exists |
| Interrupted migration | DB unchanged, retry succeeds | ⏳ Not covered by a test; guaranteed by the transaction |
| Corrupt DB | Detected, preserved, recovery path offered | ⏳ Not started |
| Read-only data directory | Clear error, no partial write | ⏳ Not started |
| Disk full mid-migration | Rollback, prior version intact | ⏳ Not covered |

Fixtures: generate a DB at version N, migrate, assert row counts and a sample of
values — not just that the migration "ran".

## 6. Writing a migration

Checklist:

```text
[ ] Is it additive where possible? (new table/column/index)
[ ] Are new columns nullable or defaulted for existing rows?
[ ] Are indexes created after data backfill?
[ ] Is the change documented in DATABASE.md?
[ ] Is there a test from every supported prior version?
[ ] Does retention still work after the change?
[ ] Does backup/export still work?
[ ] Is SCHEMA_VERSION bumped in adapters/persistence/migrations.rs?
[ ] Was the new file added to the MIGRATIONS list?
```

Large-table rules:

- Add columns with defaults carefully; SQLite rewrites tables for some alters.
- Backfill in batches outside the migration if it is slow, with a progress record.
- Create indexes after backfill to avoid long lock time.
- Never run a full `VACUUM` inside a migration transaction.

## 7. Rollback and recovery

| Situation | Action |
|---|---|
| Migration failed before commit | Nothing changed; retry after fixing the cause |
| Migration succeeded but app is broken | Restore the pre-migration backup, reinstall previous version |
| User wants to downgrade | Only supported if the older version tolerates the newer schema; otherwise refuse and explain |
| Corrupt DB after migration | Preserve the file, start fresh, offer export of readable data |

Recovery details: [`RECOVERY_PLAN.md`](RECOVERY_PLAN.md).

## 8. Settings migration (proposed)

- `schema_version` in `settings.json` drives a small migration function.
- Unknown keys are preserved (forward compatibility) but ignored.
- Invalid values fall back to defaults and are reported once.
- A settings file from a newer version is read best-effort and backed up, not
  overwritten.

## 9. Export and manifest versioning (proposed)

- Every exported report includes `schema_version`, `app_version`, and a timestamp.
- Quarantine manifests include `manifest_version`, the original path, hash,
  algorithm, size, and timestamps.
- Readers must reject unknown major versions with a clear message rather than
  guessing.

## 10. Versioning policy

| Change | Version bump |
|---|---|
| Add a table/column/index | Minor schema version |
| Backfill semantics change | Minor + test |
| Remove/rename a column with data loss | Major + backup + approval |
| Change a type with lossy conversion | Major + backup + approval |

## 11. Open questions

1. ~~`PRAGMA user_version` vs a `schema_migrations` table~~ — resolved: a
   `schema_migrations` table, because it stores per-migration checksums.
2. Is a pre-migration automatic backup mandatory or user-prompted? **Open.** Not
   needed for the additive v1; must be answered before the first destructive
   migration.
3. How long are backups retained? **Open.**
4. ~~Do we support downgrade at all?~~ — resolved: **no.** A newer schema is
   refused, and an older build does not attempt to alter it.
5. ~~Which migration library, if any, versus hand-written SQL?~~ — resolved:
   hand-written SQL embedded at compile time (ADR-018).
