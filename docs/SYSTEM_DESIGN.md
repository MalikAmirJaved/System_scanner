# System Design

> **Status:** Detailed design for a **not-yet-implemented** system. Current code is
> the Tauri scaffold described in [`../ARCHITECTURE.md`](../ARCHITECTURE.md).
> Treat every component below as proposed until `docs/FEATURES.md` says otherwise.
> **Last updated:** 2026-11-06

---

## 1. Purpose

This document specifies *how* the target architecture behaves: process model,
job lifecycle, scan pipeline, persistence flow, shutdown, and failure semantics.
Contract details live in [`../API.md`](../API.md); schema lives in
[`../DATABASE.md`](../DATABASE.md).

## 2. Process and thread model

```text
┌──────────────────────────────────────────────────────────────┐
│ Tauri process (single OS process, single user)               │
│                                                              │
│  Main/UI thread ─── WebView2 ─── React renderer              │
│        │                                                     │
│        │ IPC (invoke / event)                                │
│        ▼                                                     │
│  Tokio runtime                                               │
│    ├─ command handlers (light, non-blocking)                 │
│    ├─ job supervisor (state machine, progress coalescing)    │
│    └─ blocking pool (spawn_blocking)                         │
│          ├─ Win32 collectors                                 │
│          ├─ filesystem traversal workers                     │
│          ├─ hashing workers                                  │
│          └─ SQLite write/read tasks                          │
└──────────────────────────────────────────────────────────────┘
```

Rules:

- The UI thread never performs I/O.
- Command handlers validate and delegate; they never scan.
- Blocking syscalls always run on the blocking pool.
- One OS process. No helper service, no updater daemon, no kernel component.

## 3. Job lifecycle

```text
                 ┌──────────┐
     start ─────►│  queued  │
                 └────┬─────┘
                      │ admit (limit check)
                      ▼
                 ┌──────────┐   cancel    ┌─────────────┐
                 │ running  │────────────►│ cancelling  │
                 └────┬─────┘             └──────┬──────┘
        complete │   │ fail                    │ flushed
                 ▼   ▼                          ▼
          ┌───────────┐ ┌────────┐       ┌───────────┐
          │ completed │ │ failed │       │ cancelled │
          └───────────┘ └────────┘       └───────────┘
                                      app crash -> interrupted (on next start)
```

| State | Meaning | UI |
|---|---|---|
| `queued` | Admitted, waiting for a slot | "Waiting" |
| `running` | Workers active | Progress + cancel |
| `cancelling` | Cancel requested, flushing | "Stopping…" disabled cancel |
| `completed` | Finished; may be `partial` | Summary + skipped counts |
| `failed` | Terminal error | Error code + retry if retryable |
| `cancelled` | User/policy stopped it | Partial results retained |
| `interrupted` | Process died; reconciled at startup | "Interrupted" + discard/resume |

Transitions are one-way. A terminal state is never overwritten; a new job is
created instead.

### 3.1 Start semantics

- Duplicate start for the same scope while one is `running` → return `BUSY` with
  the existing job ID (Release 1: one storage scan at a time).
- Job ID is generated before work begins and returned immediately.
- The UI must render the new job from `queued`, not optimistically as `running`.

### 3.2 Event stream

```jsonc
{ "job_id": "…", "seq": 42, "phase": "traverse",
  "files_seen": 120345, "bytes_seen": 91234567890,
  "skipped": 12, "denied": 3, "elapsed_ms": 8123 }
```

- `seq` is monotonic per job.
- The UI drops `seq <= last_seen` and refetches status after a gap it cannot explain.
- Emission is coalesced (≤ 10/s proposed) — never one event per file.

## 4. Storage scan pipeline

```text
Discover ──► Traverse ──► Collect ──► Aggregate ──► Enrich ──► Classify ──► Present
   │            │            │             │            │           │
 roots,      bounded      normalised    incremental   hash,      evidence
 volume      workers      records       counters      signature   + policy
 caps        + cancel     + outcome     + batches                  version
```

### 4.1 Stage contracts

| Stage | Input | Output | Bounded by |
|---|---|---|---|
| Discover | Selected roots | Volume list + capabilities | root count |
| Traverse | Roots | Entry stream | worker count, depth, cancellation |
| Collect | Entry stream | `FileRecord` with outcome | channel capacity |
| Aggregate | Records | Counters, type/age/size summaries | fixed memory |
| Enrich | Candidate set | Hash/signature results | concurrency, max file size |
| Classify | Signals | Findings with confidence | rule version |
| Present | Queries | Pages, summaries | page size |

### 4.2 Traversal rules

- Iterative, not recursive, with an explicit stack.
- Reparse points: policy-driven (default: do not follow) and cycle-detected by
  volume + file ID when available.
- Each entry produces exactly one outcome: `ok`, `skipped`, `denied`,
  `disappeared`, `unsupported`, `failed`.
- One denied directory never aborts the scan.
- Cancellation checked per directory batch and per N entries.
- Metadata is captured once per entry; no re-stat loops.

### 4.3 Deduplication

1. Group by exact size (cheap).
2. Within a size group, compare a partial fingerprint (head/tail bytes).
3. Only then offer optional full hashing, bounded by size and concurrency.
4. Never mark files as duplicates from name or extension.

### 4.4 Consistency model

Best-effort point-in-time approximation. Files may change during a scan. The
summary must state: start/end time, skipped/denied counts, and that results are
approximate. Snapshot (VSS) consistency is explicitly **out of scope** initially.

## 5. Persistence flow

```text
worker records ──► bounded channel ──► batch writer (transaction)
                                          │
                                          ├─ insert file_records
                                          ├─ update scan_sessions counters
                                          └─ checkpoint job progress
```

- Batches are flush-on-size-or-time, tuned by measurement.
- A crash loses at most the in-flight batch; the session is marked `interrupted`
  at next startup.
- Reads never share the write connection's transaction.
- UI-facing queries are indexed and paged; no `SELECT *` over millions of rows.

### 5.1 Startup reconciliation

```text
open DB -> run migrations -> find sessions in running/cancelling
        -> mark interrupted -> offer discard; resume only if the stage supports it
```

## 6. Shutdown

| Trigger | Behavior |
|---|---|
| Window close during a job | Prompt; default is to cancel and flush |
| OS shutdown | Best-effort cancel; mark session `cancelled` or `interrupted` |
| Panic | `panic = "abort"` → process exits; interrupted-session recovery on next start |
| Forced kill | Same as panic; recovery path required |

No job may silently continue after the window is gone.

## 7. Security boundaries in the design

1. **WebView → IPC:** untrusted; validate everything.
2. **Command → domain:** trusted *after* validation; no OS handles cross.
3. **Domain → adapter:** capability traits with explicit outcomes.
4. **Adapter → OS:** fallible; fail closed; no elevation by default.
5. **App → filesystem (write):** identity re-verification, protected paths,
   TOCTOU handling, audit, recovery.

Details: [`THREAT_MODEL.md`](THREAT_MODEL.md), [`ACCESS_CONTROL.md`](ACCESS_CONTROL.md).

## 8. Proposed module map

```text
src-tauri/src/
  commands/          system.rs, processes.rs, storage.rs, diagnostics.rs
  app/               jobs.rs, cancellation.rs, budgets.rs, events.rs
  domain/            model/, policy/, usecase/, error.rs
  ports/             collector.rs, repository.rs, exporter.rs
  adapters/windows/  system_info.rs, processes.rs, services.rs, defender.rs
  adapters/persistence/ db.rs, migrations.rs, repo_*.rs
  adapters/filesystem/ traversal.rs, identity.rs, quarantine.rs
  reporting/         export_csv.rs, export_json.rs, redact.rs
```

## 9. Failure semantics (summary)

| Failure | Result |
|---|---|
| Field unavailable | `unavailable[]` entry with reason; snapshot marked partial |
| Directory denied | Counted, scan continues |
| File disappeared | Outcome `disappeared`, scan continues |
| DB write fails | Retry bounded; then job `failed`, prior data intact |
| Cancellation | `cancelled` within latency budget, partial data retained |
| Adapter panic | Must not occur; if it does, the job fails, app may abort |

Full matrix: [`FAILURE_MODES.md`](FAILURE_MODES.md).

## 10. Related

- [`../API.md`](../API.md) — contracts
- [`../DATABASE.md`](../DATABASE.md) — schema
- [`SCALABILITY.md`](SCALABILITY.md) — capacity model
- [`PERFORMANCE.md`](PERFORMANCE.md) — budgets
- [`ERROR_HANDLING.md`](ERROR_HANDLING.md) — error taxonomy
