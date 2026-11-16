# Database

> **Status:** **Partial.** The driver, connection setup, and forward-only migration
> runner are **implemented**, and schema **v1 contains `scan_sessions` only**. Every
> other table in §5 is still a **proposal** and does not exist in any database file.
> Do not describe a proposed table as existing.
>
> **Last updated:** 2026-11-16

---

## 1. Current state

| Item | Reality |
|---|---|
| Engine | SQLite, compiled in via `rusqlite` `bundled` |
| Driver | `rusqlite` 0.43 — `src-tauri/src/adapters/persistence/sqlite.rs` |
| Schema | **v1 implemented:** `scan_sessions` + two indexes (`src-tauri/migrations/0001_initial.sql`) |
| Migrations | **Implemented** — hand-written, forward-only, embedded, checksummed (`docs/MIGRATIONS.md`) |
| Data directory | Created at startup: `app_data_dir()/system-scanner.db` |
| Retention | None enforced; no rows are written by any feature yet |

Schema v1 is intentionally narrow. The tables in §5 for snapshots, file records,
hashes, findings, quarantine, and action history **do not exist** and will be added
by the features that write them, once retention is decided (`docs/plan.md` §12).

Runtime facts:

- Startup opens the database, applies pending migrations, logs `schema_version`,
  and stores the handle in Tauri state. Failure is logged and the app continues;
  no current panel reads from the database.
- Cutover target: `%APPDATA%\com.systemscanner.desktop\system-scanner.db`
  (roaming app data). This is the Tauri `app_data_dir()` default and is not a
  final retention decision — see §11.
- The `schema_migrations` bookkeeping table is created by the runner, not by a
  migration file.

## 2. Design goals

1. **Local single-user.** No server, no network, no multi-tenant concerns.
2. **Bounded growth.** History is retained by policy, not by accident.
3. **Honest history.** A scan's partiality, skips, and failures are recorded.
4. **Transactional.** Multi-step state changes either complete or roll back.
5. **Recoverable.** Corruption and interrupted migrations have defined recovery.
6. **Minimal personal data.** Never store file contents or full command lines.

## 3. Access-layer decision

**Decided** — see ADR-018 in [`docs/DECISIONS.md`](docs/DECISIONS.md).

| Option | Pros | Cons |
|---|---|---|
| `rusqlite` | Small, synchronous, explicit, easy to reason about with `spawn_blocking` | Manual SQL; no compile-time query checks |
| `sqlx` | Async, compile-time checked queries, migrations built in | Larger compile surface; macro/offline metadata workflow |

**Chosen:** `rusqlite` 0.43 with the `bundled` feature behind a `Mutex<Connection>`,
all database work on `spawn_blocking`, WAL mode, and hand-written versioned
migrations. Rationale: a single local writer, simple transactions, and the
smallest dependency surface. `sqlx` remains a reasonable future choice if
compile-time query checking ever outweighs its build ceremony.

## 4. Concurrency and durability rules

These pragmas are applied by `Database::open` before migrations run:

```sql
PRAGMA journal_mode = WAL;      -- one writer, concurrent readers
PRAGMA synchronous = NORMAL;    -- durable enough; full fsync per commit is too slow
PRAGMA foreign_keys = ON;
PRAGMA busy_timeout = 5000;
```

- **One write connection** guarded by a `Mutex`, owned for the process lifetime.
  This serializes writes, which is correct for a single local user.
- Commands that touch the database **must** call
  `tauri::async_runtime::spawn_blocking`; the UI thread never waits on SQLite.
- Batched inserts inside transactions sized by measured latency, not by count guess
  (B-024, not started).
- No long-running transaction may hold a UI-blocking path.
- Writes are flushed at scan checkpoints so an abort loses at most one batch.

## 5. Schema

Status per table: **Implemented** (exists in schema v1) or **Proposed** (illustrative
DDL; column names and types will change).

### 5.1 `scan_sessions` — ✅ Implemented (schema v1)

```sql
CREATE TABLE scan_sessions (
  id                TEXT PRIMARY KEY,              -- UUID, generated in Rust
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

CREATE INDEX scan_sessions_started_at ON scan_sessions(started_at DESC);
CREATE INDEX scan_sessions_module_status ON scan_sessions(module, status);
```

No `CHECK` constraints yet: status and `module` are validated in Rust, matching the
"IPC input is untrusted" rule. A later migration may add them once the value sets
are frozen.

### 5.2 `system_snapshots` — ⏳ Proposed

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

### 5.3 `process_snapshots` — ⏳ Proposed

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

### 5.4 `file_records` — ⏳ Proposed

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

### 5.5 `file_hashes` — ⏳ Proposed

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

### 5.6 `security_findings` — ⏳ Proposed

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

### 5.7 `quarantine_items` — ⏳ Proposed

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

### 5.8 `action_history` — ⏳ Proposed

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

## 6. Indexes

Indexes are added from measured query patterns, not speculatively.

**Implemented (schema v1):**

| Table | Index | Serves |
|---|---|---|
| `scan_sessions` | `(started_at DESC)` | history list |
| `scan_sessions` | `(module, status)` | per-module status queries |

**Proposed (added with their tables):**

| Table | Index | Serves |
|---|---|---|
| `file_records` | `(session_id, size_bytes DESC)` | largest files |
| `file_records` | `(session_id, extension)` | type summary |
| `file_records` | `(session_id, modified_at)` | age filters |
| `file_records` | `(session_id, size_bytes, path)` | duplicate pre-filter |
| `file_hashes` | `(digest)` | duplicate confirmation |
| `process_snapshots` | `(captured_at, pid)` | timeline |
| `security_findings` | `(classification, created_at DESC)` | findings list |
| `action_history` | `(requested_at DESC)` | audit view |

Validate with `EXPLAIN QUERY PLAN` against realistic fixtures before shipping
(B-026, not started).

## 7. Constraints and invariants

These apply to the **proposed** tables in §5.2–5.8. Schema v1 enforces only
`scan_sessions.primary key` and the self-reference on `parent_session_id`.

- `session_id` is mandatory for every file record; orphans are a bug.
- `confidence` is bounded; no NULL confidence for a finding.
- A finding must reference a rule ID and version.
- Quarantine requires a stored hash; restore verifies it before writing.
- `action_history` rows are never updated except to attach completion fields.
- Deleting a session cascades file records and hashes, never audit history.

## 8. Retention (proposed defaults, unapproved)

**Not enforced.** No feature writes rows yet, and no deletion policy runs.

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

| Scenario | Current behavior | Target |
|---|---|---|
| `SQLITE_CORRUPT` on open | The open fails, the error is logged, and the app runs without persistence | Do not delete. Rename to `app.db.corrupt-<ts>`, start fresh, surface a recovery notice, offer export of what is readable |
| Interrupted migration | Each migration runs in one transaction, so the file stays at the previous version; the next startup retries | Retry from last good version (already true) |
| Disk full | The failing statement errors and the transaction rolls back; the app logs and continues | Abort the write batch, mark session `failed` with `DISK_FULL`, keep prior history |
| DB locked | `busy_timeout = 5000` makes SQLite wait, then returns an error that is logged | Surface a diagnostic, never hang the UI |
| Read-only profile | The open fails and is logged; the app continues without persistence | Detect at startup, run in memory-only mode with a visible banner |
| Schema newer than build | `Database::open` refuses with `NewerSchema`; the file is not modified | Same |
| Released migration edited | `Database::open` refuses with `ChecksumMismatch`; the file is not modified | Same |

Only the "current behavior" column is implemented. Corruption quarantine, the
visible recovery notice, and the in-memory fallback banner are **not started**.

See [`docs/RECOVERY_PLAN.md`](docs/RECOVERY_PLAN.md).

## 10. Backup and support export

- Backup (`VACUUM INTO`) is **not implemented** (B-033).
- Support bundle never includes the raw DB; the current diagnostics bundle reads
  only the log tail.
- Details: [`docs/BACKUP_POLICY.md`](docs/BACKUP_POLICY.md).

## 11. Open questions

1. ~~`rusqlite` vs `sqlx`~~ — resolved: `rusqlite` (ADR-018).
2. Is file-level history retained at all by default, or only aggregates?
3. Where exactly do DB and quarantine live — roaming or local app data? The
   database currently uses the Tauri default (`app_data_dir()`, roaming); this is
   not a final decision.
4. Should hashes persist across session deletion to speed up rescans?
5. Is a tamper-evident hash chain required for `action_history`, or is plain
   append-only enough for a single-user app?
6. How is a schema newer than the build handled for the user — refuse to start,
   or run with a read-only view of what it understands? Currently it refuses
   persistence only (the app still starts).
