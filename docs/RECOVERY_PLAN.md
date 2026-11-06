# Recovery Plan

> **Status:** No data or state exists to recover yet. This plan defines the
> objectives and procedures to implement alongside persistence.
> **Last updated:** 2026-11-06

---

## 1. Recovery objectives (proposed)

| Objective | Target | Notes |
|---|---|---|
| RPO — scan history | ≤ 24 h (daily backup) | Acceptable to lose recent history |
| RPO — quarantine manifest | 0 (synchronous) | Must always match stored objects |
| RPO — settings | Last change | Small file, back up on change |
| RTO — app usable after corruption | ≤ 10 min | Start fresh + optional history import |
| RTO — restore a quarantined file | ≤ 2 min | Single-file operation |
| Data loss on crash | ≤ 1 write batch | Batched transactions |

No uptime SLO applies: there is no server.

## 2. Failure scenarios and procedures

### 2.1 Corrupted database

**Symptoms:** `SQLITE_CORRUPT`, `quick_check` failure, app fails to open history.

```text
1. Do not delete the file.
2. Rename to app.db.corrupt-<timestamp>.
3. Start with a fresh DB; show a banner explaining what happened.
4. Offer: open backups folder, restore latest verified backup, export readable data.
5. If readable, dump tables to a recovery export before discarding.
6. Record the event in diagnostics.
```

Never silently recreate the DB without telling the user.

### 2.2 Interrupted migration

```text
1. Migration ran in a transaction -> DB is at the previous version.
2. Restart retries; if it fails again, show the failing migration ID.
3. Offer restore from the pre-migration backup.
4. Do not attempt a partial manual fix in production code paths.
```

### 2.3 Interrupted scan (app crash / power loss)

```text
1. On startup, find sessions in running/cancelling.
2. Mark them interrupted.
3. Offer: discard partial results, or keep them labeled "partial".
4. Never resume blindly; the filesystem may have changed.
```

### 2.4 Quarantine restore failure

| Cause | Handling |
|---|---|
| Object missing/hash mismatch | Refuse restore; explain; keep record |
| Destination exists | Do not overwrite; offer rename or cancel |
| Destination path protected | Refuse; explain policy |
| Disk full | Abort; source remains quarantined |
| ACL denied | Explain; offer alternate destination |

Restore must be atomic: verify hash → write to a temp file in the destination
directory → rename into place → verify size/hash → update state.

### 2.5 Lost or damaged settings

- Fall back to defaults.
- Keep a `.bak` copy on each successful write.
- Report which settings were reset.

### 2.6 Read-only or full data directory

- Detect at startup.
- Run in memory-only mode with a persistent, visible banner.
- Disable features that require durable state.
- Never fail silently.

### 2.7 Corrupt audit chain

- Report "audit integrity unknown" in diagnostics.
- Do not delete the history.
- Offer export for analysis.
- Future actions still audited; the break is preserved, not hidden.

## 3. Recovery drills

Before a public release, perform and record:

| Drill | Pass criteria |
|---|---|
| Restore DB from `VACUUM INTO` backup | App opens, counts match |
| Restore after simulated corrupt header | Fresh start + readable export |
| Kill app mid-scan | Session marked interrupted; no lock left |
| Restore quarantined file | Byte-identical, metadata preserved |
| Restore with destination conflict | No overwrite, clear message |
| Disk-full during backup | Source intact, actionable error |

Record: date, build, steps, result, gaps.

| Date | Build | Drill | Result | Notes |
|---|---|---|---|---|
| — | — | — | Not run | No persistence yet |

## 4. User-facing guidance

Provide in-app and documentation answers for:

- "The app says my database is corrupt — what now?"
- "Where are my backups?"
- "How do I get a quarantined file back?"
- "I lost a scan — can I recover it?"
- "What happens if I uninstall?"

## 5. Open questions

1. Is history worth backing up by default, or is it disposable?
2. Should quarantine be restorable after the original file is gone? (Yes, that is
   the point — but retention must be explicit.)
3. Is a repair CLI needed, or is a fresh start + export sufficient?
4. How long are corrupt DB files retained before cleanup?
