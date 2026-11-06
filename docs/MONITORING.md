# Monitoring & Observability

> **Status:** No logging, metrics, tracing, or diagnostics exist in the code.
> **Last updated:** 2026-11-06

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

## 2. Logging (proposed)

Stack: `tracing` + `tracing-subscriber` with a rolling file appender.

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

| Setting | Proposed |
|---|---|
| File | `%LOCALAPPDATA%\com.systemscanner.desktop\logs\app.log` |
| Rotation | 10 MB per file |
| Files kept | 5 |
| Max total | 50 MB |
| Level in release | `info` (user can raise to `debug` for a session) |

## 3. Diagnostics view (Release 0 requirement)

A read-only screen showing:

- app version, Tauri version, engine/policy version;
- Windows version/build, architecture, elevation state;
- feature availability (what this machine/edition supports);
- DB path, schema version, DB size (once persistence exists);
- recent redacted log entries with a level filter;
- "Create support bundle" with a **preview** and redaction toggles.

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

- One span per job; child spans per phase (`traverse`, `persist`, `enrich`).
- `correlation_id` propagated from IPC request → job → error → UI message.
- Sampling: all jobs locally; no remote export.

## 6. Health checks

| Check | Behavior |
|---|---|
| DB open + `PRAGMA quick_check` | On startup; warn if degraded |
| Data directory writable | On startup; fall back to memory-only with banner |
| WebView2 present | Tauri handles; surface a clear error if missing |
| Migration state | Must equal expected schema version |
| Orphaned sessions | Reconciled to `interrupted` |

## 7. Crash handling

- `panic = "abort"` means no in-process recovery. The next launch detects the
  missing clean-shutdown marker and marks the session `interrupted`.
- Crash reporting is **off**. If it is ever added: opt-in, previewable payload,
  no file paths, no memory dumps.
- A local crash log with the panic message and correlation ID is acceptable and
  should be included in the support bundle.

## 8. Support bundle (planned)

```text
support-bundle-YYYYMMDD-HHMMSS.zip
  manifest.json        versions, OS build, feature flags, schema version
  logs/app.log         last N lines, redacted
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
