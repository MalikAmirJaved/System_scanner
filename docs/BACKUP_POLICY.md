# Backup Policy

> **Status:** **Pre-migration database snapshots are implemented** (B-033):
> `VACUUM INTO` into the app-data `backups/` directory, verified with
> `PRAGMA quick_check`, the newest five kept, best-effort so a failure only logs a
> warning and startup continues. Quarantine, settings, scheduled/daily backups,
> user-initiated export, and any restore control are **not implemented**. Stored
> history is still small: schema v2 holds one counters-only `scan_sessions` row
> per scan.
> **Last updated:** 2026-11-17

---

## 1. What must be backed up

| Data | Backed up | Why |
|---|---|---|
| SQLite database (`app.db`) | Yes | History, findings, audit |
| WAL/SHM sidecars | Via consistent snapshot only | Copying them naively corrupts |
| Quarantine objects + manifest | Yes, if the user wants restore | Otherwise the item is unrecoverable |
| Settings (`settings.json`) | Yes | User configuration |
| Logs | Optional, recent only | Diagnosis, not authoritative |
| Exported reports | No | User-owned files |

Never back up: temporary files, `node_modules`, build output, the installer,
or secrets.

## 2. Backup methods

### 2.1 Database — consistent snapshot (required)

```sql
VACUUM INTO 'C:\path\app-backup-YYYYMMDD-HHMMSS.db';
```

- Works while the app is running.
- Produces a single consistent file (no WAL sidecars).
- Must be verified by opening it read-only and running `PRAGMA quick_check`.

Naive file copy of `app.db` while WAL is active is **prohibited** — it can capture
a torn state.

Implemented for the pre-migration snapshot (B-033) in `Database::open` →
`backup_before_migration`: the file is named
`system-scanner-v{version:03}-{epoch_ms}.db`, so name order is age order, and a
snapshot that fails verification is deleted rather than kept.

### 2.2 Quarantine

- Copy the object files and `manifest.json` together.
- Verify each object hash after copy.
- Preserve the original-path metadata in the manifest, not in file names.

### 2.3 Settings

- Plain JSON copy; validate against `schema_version` on restore.

## 3. Frequency and retention (partially implemented)

| Trigger | Backup | Status |
|---|---|---|
| Before any pending migration | Automatic snapshot | ✅ Implemented, **best-effort** (B-033) |
| Before any non-additive migration | Mandatory automatic | ⏳ Not enforced — must become mandatory before the first destructive migration |
| Before the first destructive action in a session | Recommended | ⏳ Not implemented |
| Daily, if the app is used | Automatic rolling | ⏳ Not implemented |
| Manual "Export backup" | User-initiated | ⏳ Not implemented |

Retention: **the last 5 snapshots are kept**, counted from the file-name order
(implemented, B-033). The 30-day half of the proposed rule is not implemented.
Backups consume disk; showing their size and allowing deletion are not
implemented.

## 4. Location

Default: `%LOCALAPPDATA%\com.systemscanner.desktop\backups\`.

- Implemented location (B-033) is the database's own directory:
  `%APPDATA%\com.systemscanner.desktop\backups\`, because the database still uses
  the Tauri `app_data_dir()` roaming default. The `%LOCALAPPDATA%` choice above
  remains a proposal tied to the open data-location question
  (`docs/plan.md` §12).
- Same volume as the DB by default; warn the user that a disk failure loses both.
- Offer a user-selected external location for manual backups.
- Never write backups into the install directory.

## 5. Verification

A backup that has never been restored is a hypothesis.

```text
[ ] Open the backup read-only
[ ] PRAGMA quick_check -> ok
[ ] Table counts are plausible vs source
[ ] Spot-check a known record
[ ] Quarantine manifest hashes verify
[ ] Restore drill performed at least once per release cycle
```

Record the drill result in [`RECOVERY_PLAN.md`](RECOVERY_PLAN.md).

Implemented while taking a snapshot (B-033): the first two checks run
automatically for every pre-migration snapshot. The remaining checks — table
counts, a spot-check, quarantine hashes, and a per-release drill — are **not**
automated. One restore is exercised in `cargo test`
(`a_pending_migration_snapshots_the_database_and_the_snapshot_restores`), and it
has not been performed against a real user profile.

## 6. Encryption

- Not implemented. Local backups inherit filesystem ACLs.
- Do not claim encryption.
- If encryption is added, keys must not be stored beside the backup.

## 7. User communication

- Show backup size and location.
- Explain that deleting the app data directory removes history and quarantine.
- Explain that uninstall behavior is a documented choice (currently undecided).
- Provide a "reveal in Explorer" action rather than a hidden path.

## 8. Failure handling

| Failure | Behavior |
|---|---|
| Disk full | Abort backup, keep source intact, warn with required space |
| Permission denied | Explain, offer an alternate location |
| Verification fails | Delete the suspect backup, keep source, retry once |
| Backup older than schema | Refuse restore; explain version mismatch |

Implemented for pre-migration snapshots (B-033): a failed write or a denied
directory is logged as a warning and the migration still runs; a snapshot that
fails `PRAGMA quick_check` is deleted. There is no retry and no user-facing
message.

## 9. Open questions

1. ~~Are backups automatic by default, or user-initiated only?~~ — pre-migration
   snapshots are automatic (B-033). Whether a daily backup and a manual export
   exist by default remains **open**.
2. Should quarantine be included automatically or opt-in?
3. Is a 2 GB DB backup acceptable on low-disk machines?
4. ~~Where does retention run — on startup, on shutdown, or both?~~ — at startup,
   immediately after a snapshot, before the migration (B-033). Scheduled pruning
   remains **open**.
