# Backup Policy

> **Status:** No data exists to back up yet. This policy applies **before**
> persistence ships, so the backup path is designed in rather than bolted on.
> **Last updated:** 2026-11-06

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

### 2.2 Quarantine

- Copy the object files and `manifest.json` together.
- Verify each object hash after copy.
- Preserve the original-path metadata in the manifest, not in file names.

### 2.3 Settings

- Plain JSON copy; validate against `schema_version` on restore.

## 3. Frequency and retention (proposed)

| Trigger | Backup |
|---|---|
| Before any non-additive migration | Mandatory automatic |
| Before the first destructive action in a session | Recommended |
| Daily, if the app is used | Automatic rolling |
| Manual "Export backup" | User-initiated |

Retention: keep the last 5 automatic backups, or 30 days, whichever is smaller.
Backups consume disk; show the size and allow deletion.

## 4. Location

Default: `%LOCALAPPDATA%\com.systemscanner.desktop\backups\`.

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

## 9. Open questions

1. Are backups automatic by default, or user-initiated only?
2. Should quarantine be included automatically or opt-in?
3. Is a 2 GB DB backup acceptable on low-disk machines?
4. Where does retention run — on startup, on shutdown, or both?
