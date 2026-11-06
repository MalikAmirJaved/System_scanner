# Database

> **Status:** **Not implemented.** No database, driver, migration, or schema exists
> in this repository. Everything below is a **proposal** derived from
> [`docs/plan.md`](docs/plan.md) §4 and must be validated before implementation.
>
> **Last updated:** 2026-11-06

---

## 1. Current state

| Item | Reality |
|---|---|
| Engine | None |
| Driver | None (`Cargo.toml` has only `tauri`) |
| Schema | None |
| Migrations | None |
| Data directory | None created |
| Retention | None |

Do not describe any table in this document as existing. When the first migration
lands, replace this section with the applied schema version and file path.

## 2. Design goals

1. **Local single-user.** No server, no network, no multi-tenant concerns.
2. **Bounded growth.** History is retained by policy, not by accident.
3. **Honest history.** A scan's partiality, skips, and failures are recorded.
4. **Transactional.** Multi-step state changes either complete or roll back.
5. **Recoverable.** Corruption and interrupted migrations have defined recovery.
6. **Minimal personal data.** Never store file contents or full command lines.

## 3. Access-layer decision (open)

| Option | Pros | Cons |
|---|---|---|
| `rusqlite` | Small, synchronous, explicit, easy to reason about with `spawn_blocking` | Manual SQL; no compile-time query checks |
| `sqlx` | Async, compile-time checked queries, migrations built in | Larger compile surface; macro/offline metadata workflow |

**Recommendation:** `rusqlite` with an explicit repository layer, all DB calls on
`spawn_blocking`, WAL mode, and hand-written versioned migrations. Rationale: a
single local writer, simple transactions, and the smallest dependency surface.
This is a **proposal**; record the final choice in [`docs/DECISIONS.md`](docs/DECISIONS.md).

## 4. Concurrency and durability rules (proposed)

```sql
PRAGMA journal_mode = WAL;      -- one writer, concurrent readers
PRAGMA synchronous = NORMAL;    -- durable enough; full fsync per commit is too slow
PRAGMA foreign_keys = ON;
PRAGMA busy_timeout = 5000;
```

- One write connection in a dedicated task; reads may use a small pool.
- Batched inserts inside transactions sized by measured latency, not by count guess.
- No long-running transaction may hold a UI-blocking path.
- Writes are flushed at scan checkpoints so an abort loses at most one batch.

## 5. Proposed schema

> Illustrative DDL. Column names/types will change. Status: **Proposed**.

### 5.1 `scan_sessions`

```sql
CREATE TABLE scan_sessions (
  id                TEXT PRIMARY KEY,              -- UUID
  module            TEXT NOT NULL,                 -- system|processes|storage|security|network|diagnostics
  roots_json        TEXT NOT NULL DEFAULT '[]',    -- requested scope (validated in Rust)
  options_json      TEXT NOT NULL DEFAULT '{}',    -- concurrency, limits, policies
  status            TEXT NOT NULL,                 -- queued|running|cancelling|completed|failed|cancelled|interrupted
  started_at        INTEGER NOT NULL,              -- unix ms UTC
  finished_at       INTEGER,
  engine_version    TEXT NOT NULL,
  policy_version    TEXT NOT NULL,
  files_seen        INTEGER NOT NULL DEFAULT 0,
  files_skipped     INTEGER NOT NULL DEFAULT 0,
  files_denied      INTEGER NOT NULL DEFAULT 0,
  files_failed      INTEGER NOT NULL DEFAULT 0,
  bytes_seen        INTEGER NOT NULL DEFAULT 0,
  cancel_reason     TEXT,
  error_code        TEXT,                          -- joins the app error taxonomy
  parent_session_id TEXT REFERENCES scan_sessions(id)
);
```

### 5.2 `system_snapshots`

```sql
CREATE TABLE system_snapshots (
  id           TEXT PRIMARY KEY,
  captured_at  INTEGER NOT NULL,
  os_json      TEXT NOT NULL,
  cpu_json     TEXT NOT NULL,
  memory_json  TEXT NOT NULL,
  disks_json   TEXT NOT NULL,
  network_json TEXT NOT NULL,
  partial      INTEGER NOT NULL DEFAULT 0        -- 1 if any field was unavailable
);
```

### 5.3 `process_snapshots`

```sql
CREATE TABLE process_snapshots (
  id                TEXT PRIMARY KEY,
  captured_at       INTEGER NOT NULL,
  pid               INTEGER NOT NULL,
  parent_pid        INTEGER,
  identity          TEXT,                        -- start time + PID, for stable identity
  name              TEXT NOT NULL,
  executable_path   TEXT,                        -- may be NULL when access denied
  user_sid          TEXT,
  working_set_bytes INTEGER,
  cpu_time_ms       INTEGER,
  access_limited    INTEGER NOT NULL DEFAULT 0
);
```

### 5.4 `file_records`

```sql
CREATE TABLE file_records (
  id             INTEGER PRIMARY KEY,
  session_id     TEXT NOT NULL REFERENCES scan_sessions(id) ON DELETE CASCADE,
  path           TEXT NOT NULL,
  volume_id       TEXT,
  file_id         TEXT,                          -- NTFS file ID when available
  size_bytes      INTEGER NOT NULL,
  modified_at     INTEGER,
  extension       TEXT,
  attributes      INTEGER,
  outcome         TEXT NOT NULL,                 -- ok|skipped|denied|disappeared|unsupported|failed
  hash_status     TEXT NOT NULL DEFAULT 'none'   -- none|partial|complete|failed
);
```

### 5.5 `file_hashes`

```sql
CREATE TABLE file_hashes (
  id            INTEGER PRIMARY KEY,
  file_record_id INTEGER NOT NULL REFERENCES file_records(id) ON DELETE CASCADE,
  algorithm     TEXT NOT NULL,                   -- sha256|blake3
  digest        TEXT NOT NULL,
  size_bytes    INTEGER NOT NULL,
  computed_at   INTEGER NOT NULL,
  algorithm_version TEXT NOT NULL
);
```

### 5.6 `security_findings`

```sql
CREATE TABLE security_findings (
  id             TEXT PRIMARY KEY,
  session_id     TEXT REFERENCES scan_sessions(id) ON DELETE SET NULL,
  subject_type   TEXT NOT NULL,                  -- file|process|service|startup|system
  subject_ref    TEXT NOT NULL,
  classification TEXT NOT NULL,                  -- known|suspicious|unknown|not_scanned
  confidence     REAL NOT NULL CHECK (confidence BETWEEN 0 AND 1),
  rule_id        TEXT NOT NULL,
  rule_version   TEXT NOT NULL,
  engine_version TEXT NOT NULL,
  evidence_json  TEXT NOT NULL,                  -- provenance, never raw file contents
  created_at     INTEGER NOT NULL,
  user_disposition TEXT                          -- open|acknowledged|quarantined|restored|ignored
);
```

### 5.7 `quarantine_items`

```sql
CREATE TABLE quarantine_items (
  id               TEXT PRIMARY KEY,
  original_path    TEXT NOT NULL,
  stored_name      TEXT NOT NULL,                -- opaque name under quarantine/
  size_bytes       INTEGER NOT NULL,
  sha256           TEXT NOT NULL,
  source_volume_id TEXT,
  source_file_id   TEXT,
  quarantined_at   INTEGER NOT NULL,
  state            TEXT NOT NULL,                -- quarantined|restored|purged|failed
  restored_at      INTEGER,
  integrity_verified INTEGER NOT NULL DEFAULT 0
);
```

### 5.8 `action_history`

```sql
CREATE TABLE action_history (
  id            TEXT PRIMARY KEY,
  action        TEXT NOT NULL,                   -- terminate_process|stop_service|quarantine|restore|delete
  target        TEXT NOT NULL,
  confirmation  TEXT NOT NULL,                   -- what the user approved
  requested_at  INTEGER NOT NULL,
  completed_at  INTEGER,
  result        TEXT NOT NULL,                   -- succeeded|denied|failed|cancelled
  error_code    TEXT,
  actor         TEXT NOT NULL DEFAULT 'local-user',
  prev_hash     TEXT,
  entry_hash    TEXT NOT NULL                    -- tamper-evident chain
);
```

## 6. Proposed indexes

Indexes are added from measured query patterns, not speculatively. Starting set:

| Table | Index | Serves |
|---|---|---|
| `scan_sessions` | `(module, started_at DESC)` | history list |
| `file_records` | `(session_id, size_bytes DESC)` | largest files |
| `file_records` | `(session_id, extension)` | type summary |
| `file_records` | `(session_id, modified_at)` | age filters |
| `file_records` | `(session_id, size_bytes, path)` | duplicate pre-filter |
| `file_hashes` | `(digest)` | duplicate confirmation |
| `process_snapshots` | `(captured_at, pid)` | timeline |
| `security_findings` | `(classification, created_at DESC)` | findings list |
| `action_history` | `(requested_at DESC)` | audit view |

Validate with `EXPLAIN QUERY PLAN` against realistic fixtures before shipping.

## 7. Constraints and invariants

- `session_id` is mandatory for every file record; orphans are a bug.
- `confidence` is bounded; no NULL confidence for a finding.
- A finding must reference a rule ID and version.
- Quarantine requires a stored hash; restore verifies it before writing.
- `action_history` rows are never updated except to attach completion fields.
- Deleting a session cascades file records and hashes, never audit history.

## 8. Retention (proposed defaults, unapproved)

| Data | Default | Notes |
|---|---|---|
| System snapshots | 30 days | Bounded rollup after 7 days |
| Process snapshots | 7 days | High volume |
| File records | Until scan deleted | Manual + automatic cap |
| Hashes | With file record | Reused across scans by digest |
| Security findings | 90 days | Required for user disposition history |
| Action history | Until app uninstall | Needed for restore/audit |
| Quarantine objects | Until purged by user | Never auto-purged |

Retention must be visible in the UI and must never delete data needed to restore
quarantine items. See [`docs/DATA_PRIVACY.md`](docs/DATA_PRIVACY.md).

## 9. Corruption and recovery

| Scenario | Handling |
|---|---|
| `SQLITE_CORRUPT` on open | Do not delete. Rename to `app.db.corrupt-<ts>`, start fresh, surface a recovery notice, offer export of what is readable |
| Interrupted migration | Migrations run in a transaction with a schema-version check; retry from last good version |
| Disk full | Abort the write batch, mark session `failed` with `DISK_FULL`, keep prior history |
| DB locked | `busy_timeout` + retry with backoff; surface a diagnostic, never hang the UI |
| Read-only profile | Detect at startup, run in memory-only mode with a visible banner |

See [`docs/RECOVERY_PLAN.md`](docs/RECOVERY_PLAN.md).

## 10. Backup and support export

- Backup: `VACUUM INTO '<path>'` produces a consistent snapshot while the app runs.
- Support export: redacted copy, never the raw DB, with paths normalized and
  user names removed.
- Details: [`docs/BACKUP_POLICY.md`](docs/BACKUP_POLICY.md).

## 11. Open questions

1. `rusqlite` vs `sqlx` (blocking decision).
2. Is file-level history retained at all by default, or only aggregates?
3. Where exactly do DB and quarantine live — roaming or local app data?
4. Should hashes persist across session deletion to speed up rescans?
5. Is a tamper-evident hash chain required for `action_history`, or is plain
   append-only enough for a single-user app?
