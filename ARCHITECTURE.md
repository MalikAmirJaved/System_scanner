# Architecture

> **Status:** Current state + target design · **Last updated:** 2026-11-13
>
> The current implementation is a Tauri scaffold with one vertical slice: a
> read-only system snapshot. The target design below is **proposed** and must be
> adopted incrementally, feature by feature. Neither section is a claim that the
> target exists.

---

## 1. Current architecture (implemented)

```text
┌────────────────────────────────────────────────────────┐
│ WebView (React 19 + TS)                                │
│   src/main.tsx  -> React root                          │
│   src/App.tsx   -> welcome screen + data panels        │
│   src/components/ -> BackendStatus, SystemOverview,    │
│                      ProcessList, Diagnostics          │
│   src/services/ -> typed IPC adapters                   │
│   src/App.css   -> design tokens + styles               │
│   One invoke call site, no OS access                    │
└───────────────────────┬─────────────────────────────────┘
                        │ invoke(get_app_info |
                        │   get_system_snapshot | list_processes |
                        │   get_diagnostics)
┌───────────────────────▼─────────────────────────────────┐
│ Tauri 2 runtime                                         │
│   src-tauri/src/lib.rs  -> Builder + handler            │
│   commands/            -> validate + delegate           │
│   dto/                 -> versioned wire DTOs           │
│   domain/              -> platform-neutral              │
│   ports/               -> narrow capabilities           │
│   adapters/windows/    -> windows-sys only              │
│   adapters/persistence/-> rusqlite + migrations         │
│   adapters/filesystem/ -> bounded traversal prototype   │
│   ipc.rs / error.rs    -> envelope + taxonomy           │
│   logging.rs           -> bounded, redacted file logs   │
│   redact.rs            -> path/identity scrubbing       │
│   startup.rs           -> last-resort fatal dialog      │
│   capabilities/default.json -> ["core:default"]         │
│   CSP: default-src 'self'; no remote origins            │
└───────────────────────┬─────────────────────────────────┘
                        │
┌───────────────────────▼─────────────────────────────────┐
│ Rust std + tauri + serde/thiserror + tracing + rusqlite   │
│ + windows-sys: four read-only commands, bounded local     │
│ logs, on-request redacted diagnostics, an open-but-unused │
│ schema-v1 database, and an unwired traversal prototype;   │
│ no storage scan, jobs, or events                          │
└───────────────────────────────────────────────────────────┘
```

**Properties that already hold**

- The renderer cannot reach the OS: no shell/fs/http plugin is enabled and no
  capability grants one. It can only invoke the four read-only commands.
- Diagnostics are generated on request, stay local, and return a redacted preview
  rather than a path or an uploaded payload.
- The capability surface is `core:default` only.
- CSP blocks remote script/style/font/connect origins.
- The release binary is LTO-optimized, stripped, and `panic = "abort"`.
- The bundle is a single NSIS installer.
- Every command returns the `CommandResult` envelope; the frontend has exactly one
  `invoke` call site and unwraps the envelope there.
- Errors have a stable code, a safe message, a retryability flag, and a
  correlation ID.
- Win32 calls exist only in `adapters/windows/`. `domain/` compiles and is tested
  without `windows-sys`; the snapshot and process inventory are reached through
  narrow `ports/` traits.
- The snapshot reports unavailable groups and leaf fields with a reason instead
  of fabricating zero or empty values.
- `list_processes` validates `offset`/`limit` in Rust, orders by PID so pages are
  stable, and keeps rows Windows refused to open with a per-field reason.
- A failed startup shows a native dialog with a redacted detail and next steps,
  then exits 1; a render failure is caught by a boundary that offers a reload.
- Opening the database is confined to `adapters/persistence/`. `domain/` still
  imports no SQL and no driver type. The file is opened at startup, migrations
  run, and the handle is stored in Tauri state.
- Filesystem enumeration is confined to `adapters/filesystem/`. The prototype
  streams entries through a bounded channel, checks cancellation between entries,
  reports depth/entry limits, and never follows reparse points. `domain/` holds
  only the outcome vocabulary and counters.

**Properties that do not hold yet**

- No events, job registry, or cancellation at the job level.
- No command reads or writes the database; schema v1 holds no user rows.
- No IPC command, UI, or persistence path consumes the traversal prototype.
- No startup, service, storage summary, or security collectors.
- No navigation or routing.
- Tests cover the IPC contract, the snapshot/process/diagnostics domain and DTOs,
  both Windows adapters' structural invariants, paging boundaries, logging,
  redaction, startup message text, migration ordering/checksums, schema creation,
  WAL mode, schema-version refusal, traversal outcomes/budgets/cancellation/long
  and Unicode paths/junction loops, and the four UI panels plus the error
  boundary. Scan sessions, file-record persistence, and UI wiring for traversal
  are untested because they do not exist.
- CI exists as a file but has never run on GitHub.
- CI exists as a file but has never run on GitHub.

## 2. Target architecture (proposed)

A modular monolith: one desktop application, one local Rust engine, explicit
domain boundaries. No microservices, no background service, no cloud component
in the baseline.

```text
React + TypeScript UI (Tauri WebView)
  ├─ app shell, routing, providers
  ├─ feature pages (system, processes, storage, security, network, diagnostics)
  ├─ shared components, virtualized tables
  ├─ typed command/event adapters (services)
  └─ UI state only — no OS access, no security decisions
          │  typed commands, events, DTOs
          ▼
Tauri application boundary
  ├─ narrow command surface + input validation
  ├─ capability/permission configuration
  ├─ session + job lifecycle management
  └─ event serialization
          │
          ▼
Rust core (no Windows imports)
  ├─ application/  orchestrator: jobs, cancellation, progress, budgets
  ├─ domain/       models, policies, use cases, error types
  ├─ ports/        traits: collectors, repositories, exporters
  └─ policies/     privilege, protected paths, retention, authorization
          │
          ├──────────────┬──────────────┬──────────────┐
          ▼              ▼              ▼              ▼
   adapters/windows  adapters/persistence  adapters/filesystem  reporting
   Win32 / windows-rs   SQLite repos        safe traversal       CSV/JSON export
```

### Module responsibilities

| Module                    | Owns                                                              | Must not                              |
| ------------------------- | ----------------------------------------------------------------- | ------------------------------------- |
| `commands/`             | Deserialize and validate input, authorize, delegate, map errors   | Contain scanning algorithms or SQL    |
| `app/`                  | Job registry, lifecycle, cancellation tokens, progress coalescing | Know Win32 details                    |
| `domain/`               | Entities, value objects, policies, use-case logic                 | Import `windows`, `tauri`, or SQL |
| `ports/`                | Narrow traits for OS, storage, export                             | Leak platform handles                 |
| `adapters/windows/`     | Win32/WMI/registry/Defender access, `unsafe` isolation          | Leak raw handles upward               |
| `adapters/persistence/` | SQLite repositories, migrations                                   | Contain business rules                |
| `adapters/filesystem/`  | Traversal prototype (read-only); identity checks and quarantine storage are not started | Contain business rules; follow reparse points; write |
| `reporting/`            | Export formats, redaction                                         | Read the OS directly                  |
| `src/services/` (FE)    | Typed `invoke()` wrappers, event subscriptions                  | Format data for display               |
| `src/features/` (FE)    | Screens, feature state, user flows                                | Call `invoke()` directly            |

### Dependency direction (enforced by review)

```text
features -> services -> (IPC) -> commands -> app -> domain <- ports <- adapters
```

- `domain` depends on nothing outward; adapters depend on `domain`/`ports`.
- No cycles. No global mutable engine singleton.
- Cross-module calls use explicit request/result types.
- `unsafe` appears only in `adapters/windows` (and only where a safe wrapper
  cannot express the operation), each block documented with its invariant.

Architectural exceptions require an ADR in [`docs/DECISIONS.md`](docs/DECISIONS.md).

## 3. Runtime and concurrency model (proposed)

| Concern            | Design                                                                                     |
| ------------------ | ------------------------------------------------------------------------------------------ |
| UI thread          | Never blocks on I/O; renders from DTO state                                                |
| Rust async runtime | Tokio (via Tauri) for command handlers and coordination                                    |
| Blocking work      | `spawn_blocking` for Win32 calls and filesystem traversal                                |
| Scan workers       | Bounded pool, configurable concurrency, backpressure via bounded channels                  |
| Cancellation       | `CancellationToken` per job, checked at enumeration/enrichment/persist/report boundaries |
| Progress           | Throttled/coalesced events keyed by job ID with monotonic sequence                         |
| Persistence        | Batched transactions on a dedicated write path; reads paginated                            |
| Shutdown           | Jobs receive cancel, flush, then mark session `interrupted`                              |

## 4. Key boundaries

### 4.1 IPC boundary (security + performance)

Tauri IPC is treated as untrusted input and a throughput bottleneck. Commands are
narrow and typed; large result sets are paged or aggregated in Rust. Contract
details live in [`API.md`](API.md).

### 4.2 Domain/OS boundary

Windows behavior is fallible and sometimes slow. Every adapter returns explicit
outcomes (`Ok`, `AccessDenied`, `Unsupported`, `Disappeared`, `Timeout`) rather
than collapsing failure into an empty default. This is what makes "unknown" a
first-class result in the UI.

### 4.3 Persistence boundary

Domain workflows never embed SQL. Repositories own queries; migrations are
versioned and forward-only. Only `adapters/persistence/` may open the database.
The connection is synchronous, so any command that uses it must run on
`spawn_blocking`. Schema v1 (`scan_sessions`) and the runner are implemented; no
repository or command uses them yet. See [`DATABASE.md`](DATABASE.md) and
[`docs/MIGRATIONS.md`](docs/MIGRATIONS.md).

### 4.4 Privilege boundary

The app runs unelevated by default. Any operation requiring elevation is a
separate, explicitly authorized command that fails safely when elevation is
denied. It never shares a code path with read-only collection.

### 4.5 Filesystem boundary

Only `adapters/filesystem/` may enumerate directories. `domain/` decides what an
entry *means* (kind, skip reason, outcome, limits, counters) and performs no I/O;
the adapter reads metadata and `read_dir` and streams results.

The current capability is a **read-only traversal prototype**:

- roots and limits are validated before any thread starts (absolute paths, root
  and worker and capacity caps);
- a fixed worker pool pulls directory jobs from one work queue; entries are
  pushed through a bounded channel, so a slow consumer slows the walk instead of
  growing memory;
- cancellation and an optional entry budget stop the walk between entries; a
  budget stop is reported as `truncated`, never as a complete scan;
- every observed entry gets exactly one outcome (`ok`, `skipped`, `denied`,
  `disappeared`, `unsupported`, `failed`), and nothing is dropped silently;
- reparse points (symlinks, junctions, other Windows reparse attributes,
  including cloud placeholders) are reported and **never followed**, so a
  junction loop cannot hang the walk. Following them would need a visited set
  keyed by volume + file ID.

The prototype writes nothing: no database row, no log line carrying a path, no
IPC response, no UI state. Wiring it to scan jobs (B-025) and a batch writer
(B-024) is a separate change and requires the retention decision in
[`docs/plan.md`](docs/plan.md) §12.

## 5. Data flow: a scan (proposed)

```text
User selects roots
  -> UI invokes start_scan(request) -> { jobId }         (returns immediately)
  -> commands/ validates paths, size limits, concurrency
  -> app/ registers job, spawns supervised workers
  -> adapters/filesystem enumerates (bounded queue, cancellation)
  -> domain/ aggregates incrementally
  -> adapters/persistence writes batches in transactions
  -> events: scan://progress { jobId, seq, counts, phase }
  -> UI subscribes, renders virtualized summaries
  -> on completion: scan://completed { jobId, summary, partiality }
  -> UI queries pages/summaries via separate paged commands
```

The UI never receives the full file list. Aggregates and pages are queried on
demand.

## 6. Architectural constraints

1. **Read-only before control.** No feature may add a write operation before its
   read-only counterpart is complete and tested.
2. **Bounded everything.** Every queue, result set, log, and event stream has a
   documented limit.
3. **Explicit partiality.** Scan and collection results always report skipped,
   inaccessible, and failed counts; silence is not success.
4. **No shelling out** for core functionality. Use documented Windows APIs.
5. **No silent network.** A dependency that can make network calls must be
   justified, disabled by default, and disclosed.
6. **Versioned contracts.** Persisted and exported schemas carry a version.

## 7. Repository layout evolution

Target layout (create directories only when a feature needs them):

```text
frontend/src/{app,features,components,services,stores}
src-tauri/src/{commands,app,domain,ports,adapters/{windows,persistence,filesystem},reporting}
src-tauri/{capabilities,migrations}
docs/
```

Migration steps are tracked in [`docs/BACKLOG.md`](docs/BACKLOG.md) and recorded
in [`docs/DECISIONS.md`](docs/DECISIONS.md).

## 8. Related documents

- [`docs/SYSTEM_DESIGN.md`](docs/SYSTEM_DESIGN.md) — state machines and detailed flows
- [`API.md`](API.md) — IPC contracts
- [`DATABASE.md`](DATABASE.md) — schema
- [`docs/SCALABILITY.md`](docs/SCALABILITY.md) — scaling model
- [`docs/THREAT_MODEL.md`](docs/THREAT_MODEL.md) — trust boundaries and adversaries
- [`AI_ARCHITECTURE_RULES.md`](AI_ARCHITECTURE_RULES.md) — enforceable rules
