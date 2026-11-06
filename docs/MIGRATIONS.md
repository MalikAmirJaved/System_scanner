# Migrations

> **Status:** No database, schema, or migration system exists. This document
> defines the required procedure **before** persistence lands.
> **Last updated:** 2026-11-06

---

## 1. Scope

Migrations apply to:

| Artifact | Versioned by |
|---|---|
| SQLite schema | `PRAGMA user_version` (proposed) |
| Settings file | `schema_version` field |
| Exported reports | `schema_version` in the export envelope |
| Quarantine manifest | `manifest_version` |
| App data directory layout | app version + documented note |

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

## 3. Proposed layout

```text
src-tauri/migrations/
  0001_initial.sql
  0002_add_file_hashes.sql
  0003_add_action_history.sql
```

Rules:

- `NNNN_snake_case_description.sql`, zero-padded, strictly increasing.
- One logical change per migration.
- No `IF NOT EXISTS` masking of unexpected state in a new migration.
- Applied migrations are checksummed; a changed checksum on a released migration
  is a build error.

## 4. Runner behavior (proposed)

```text
begin immediate
  read user_version
  if user_version > code_version -> refuse to start, warn "newer app version"
  for each migration > user_version:
      execute
      set user_version
commit
on error:
  rollback
  keep DB unchanged
  show recoverable error with the failing migration id
```

Additional rules:

- Take a backup (`VACUUM INTO`) before a migration that is not purely additive.
- Show progress for long migrations; do not block the UI without feedback.
- Never run migrations concurrently with an active scan.
- After migration, run `PRAGMA quick_check` (or `integrity_check` for major
  versions) and report failure.

## 5. Test matrix

| Starting state | Expected |
|---|---|
| Empty / new install | Fresh schema at latest version |
| Each released version | Upgrades to latest, data intact |
| Interrupted migration | DB unchanged, retry succeeds |
| DB from a newer app version | Refuses to run, explains, does not downgrade |
| Corrupt DB | Detected, preserved, recovery path offered |
| Read-only data directory | Clear error, no partial write |
| Disk full mid-migration | Rollback, prior version intact |

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
[ ] Is the schema version bumped in one place?
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

## 8. Settings migration

- `schema_version` in `settings.json` drives a small migration function.
- Unknown keys are preserved (forward compatibility) but ignored.
- Invalid values fall back to defaults and are reported once.
- A settings file from a newer version is read best-effort and backed up, not
  overwritten.

## 9. Export and manifest versioning

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

1. `PRAGMA user_version` vs a `schema_migrations` table (checksums are easier with
   a table).
2. Is a pre-migration automatic backup mandatory or user-prompted?
3. How long are backups retained?
4. Do we support downgrade at all?
5. Which migration library, if any, versus hand-written SQL?
