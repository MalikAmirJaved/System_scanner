# Monitoring & Observability

> **Status:** Local structured logging and an in-app diagnostics preview exist
> (redacted, bounded, on request). Metrics, a saved-file support bundle, and a
> log viewer/filter do not exist yet.
> **Last updated:** 2026-11-14

---

## 1. Principles

1. **Local by default.** Diagnostics serve the user and the support engineer, on
   the user's machine.
2. **No telemetry.** Nothing is uploaded unless the user explicitly exports and
   shares a bundle.
3. **Redacted by construction.** Sensitive fields are excluded at the logging
   call site, not scrubbed later.
4. **Bounded.** Log rotation and retention are mandatory.
5. **Useful under failure.** The most valuable logs are from crashes, denials, and
   cancellations.

## 2. Logging (implemented, Release 0)

Stack: `tracing` + `tracing-subscriber`, writing through the size-rotating
writer in `src-tauri/src/logging.rs`. Commands log their outcome, stable error
code, and correlation ID; payloads and IPC inputs are not logged.

Output is the default text `tracing` format today. The JSONL schema below is the
target for the support bundle (B-006), not the current file format.

| Level | Use |
|---|---|
| `error` | Operation failed; user-visible consequence |
| `warn` | Recoverable anomaly: denied path, retry, partial result |
| `info` | Lifecycle: job started/completed/cancelled, migration applied |
| `debug` | Stage transitions, batch sizes, timings (off by default) |
| `trace` | Per-entry detail; never enabled in release builds by default |

### 2.1 Record schema

```jsonc
{
  "ts": "2026-11-06T12:00:00.123Z",
  "level": "info",
  "target": "app.jobs",
  "msg": "job completed",
  "job_id": "…",
  "session_id": "…",
  "phase": "traverse",
  "duration_ms": 8123,
  "files_seen": 120345,
  "denied": 3,
  "skipped": 12,
  "correlation_id": "…"
}
```

### 2.2 Redaction rules

Never log:

- full file paths by default (log basename or a hash; full path only at `debug`
  after explicit opt-in);
- file contents, command lines, environment variables;
- usernames, SIDs, machine names, IP addresses;
- tokens, keys, secrets.

Errors surfaced to the UI use a correlation ID; the detailed cause stays local.

### 2.3 Rotation and retention

| Setting | Implemented |
|---|---|
| File | `%LOCALAPPDATA%\com.systemscanner.desktop\logs\system-scanner.log` (resolved by Tauri `app_log_dir()`) |
| Rotation | 10 MB per file |
| Files kept | 5 |
| Max total | 50 MB |
| Level in release | `info` (no runtime toggle yet) |

## 3. Diagnostics view (implemented, Release 0 subset)

The dashboard has an on-request diagnostics panel
(`src/components/Diagnostics.tsx`). It shows:

- app and Tauri versions, plus the DTO schema version;
- Windows name/version/build, architecture, and elevation state;
- log file name, aggregate size, file count, and last write time;
- unavailable fields with their stable reason codes;
- a redacted support-bundle preview with an explicit copy action.

It does **not** show feature availability, database statistics or schema version
(the database file exists but holds no user rows and no command reads it), a
log-level filter, or redaction toggles. It never writes a file, and loading it is
a user action, so browsing the dashboard does not read the log.

The command is `get_diagnostics`; see [`API.md`](../API.md) §3.4.

## 4. Metrics (local)

No Prometheus endpoint. Metrics are computed locally and shown in the UI or
recorded in the DB:

| Metric | Where |
|---|---|
| Launch to dashboard ready | log + diagnostics |
| Refresh duration | event payload |
| Scan throughput (entries/s) | job progress |
| Peak memory (RSS) | sampled per job (proposed) |
| Cancellation latency | job completion record |
| Scan outcomes (completed/cancelled/failed) | local counter table |
| Crash-free sessions | local marker written at clean shutdown |
| DB size, row counts | diagnostics |

These are visible to the user and never uploaded.

## 5. Tracing (proposed)

Command outcomes are logged as events today. Job spans are not implemented:

- One span per job; child spans per phase (`traverse`, `persist`, `enrich`).
- `correlation_id` propagated from IPC request → job → error → UI message.
- Sampling: all jobs locally; no remote export.

## 6. Health checks

| Check | Behavior |
|---|---|
| DB open | On startup: creates the directory, configures pragmas, applies migrations, logs `database ready` with `schema_version`. A failure logs a warning and the app continues without persistence |
| `PRAGMA quick_check` | ⏳ Not implemented |
| Data directory writable | On startup; the open failure path logs and disables persistence. A visible memory-only banner is not implemented |
| WebView2 present | Tauri handles; surface a clear error if missing |
| Migration state | Must equal expected schema version; a newer schema or changed checksum refuses to open |
| Orphaned sessions | 🟡 Partial — open rows are marked `interrupted` at startup and the count is logged as a warning; no metric, no UI |

## 7. Crash handling

- `panic = "abort"` means no in-process recovery. The next launch detects the
  missing clean-shutdown marker and marks the session `interrupted`.
- A startup failure that prevents a window shows a native message box with the
  version, a redacted error detail, and next steps, then exits non-zero
  (`startup.rs`). A render failure is caught by the renderer boundary, which
  offers a reload instead of a blank window.
- Crash reporting is **off**. If it is ever added: opt-in, previewable payload,
  no file paths, no memory dumps.
- A local crash log with the panic message and correlation ID is acceptable and
  should be included in the support bundle.

## 8. Support bundle (preview implemented, file export planned)

The current bundle is plain text. It contains app/Tauri versions, OS details,
elevation, log size, and at most 256 KiB of the log tail, starting on a whole
line. It is previewed in the app before the user copies it; nothing is uploaded
or written to disk by the command.

Redaction is best-effort and runs over the finished text. It replaces
user-profile and other absolute paths, UNC paths, e-mail addresses, IPv4/IPv6
addresses, MAC addresses, SIDs, and the current account and machine names. See
`src-tauri/src/redact.rs`; the limits are recorded as KI-043.

Not implemented:

- a saved `.zip` or `.txt` export with a file-picker flow;
- a `manifest.json` / `diagnostics.json` split;
- DB statistics and job outcomes (schema v1 exists but no rows are written);
- a redaction toggle or rule preview.

The target shape remains:

```text
support-bundle-YYYYMMDD-HHMMSS.zip
  manifest.json        versions, OS build, feature flags, schema version
  logs/system-scanner.log  last N bytes, redacted
  diagnostics.json     availability, DB stats, job outcomes
  NOT INCLUDED: file contents, full paths, quarantine objects, the raw DB
```

The user must be able to inspect the bundle before sharing it.

## 9. Alerting

There is no pager and no server. "Alerting" means:

- the UI surfaces degraded states (DB corrupt, read-only profile, repeated
  failures) with a next step;
- the support bundle captures enough to diagnose without a remote session.

## 10. Open questions

1. Is in-app log viewing enough, or is a log file viewer needed?
2. Should per-job peak memory be sampled via `GetProcessMemoryInfo`?
3. Is a local crash marker sufficient to prove crash-free sessions?
4. What exact redaction applies to paths in `info` logs?
