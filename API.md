# API

> **Status:** **IPC envelope implemented; two commands implemented.** The typed
> envelope (`CommandResult`, `AppError`), the error taxonomy, `get_app_info`, and
> the read-only `get_system_snapshot` exist and are covered by tests. Every other
> command in this document is **proposed** and must not be described as available.
>
> **Last updated:** 2026-11-13 · **Related:** [`ARCHITECTURE.md`](ARCHITECTURE.md)

---

## 0. What "API" means here

There is no network service and there will not be one for core functionality.
The only API is the **Tauri IPC boundary** between the React renderer and the
Rust core.

```text
React (untrusted UI)  ──invoke(command, payload)──►  Rust commands
React (untrusted UI)  ◄──── event(event, payload)────  Rust app/orchestrator
```

Consequences:

- IPC input is untrusted and validated in Rust.
- IPC output is a stable DTO, never a raw platform handle or internal struct.
- IPC volume is a performance constraint: page or aggregate large results.

## 1. Naming and conventions

| Element | Convention | Example |
|---|---|---|
| Command | `snake_case` verb + noun, grouped by domain | `get_system_snapshot` |
| Event | `<domain>:<thing>` | `scan:progress` |
| DTO field | **`camelCase` on the wire** | `startedAt` |
| Error code | `snake_case` string enum | `access_denied` |
| ID | opaque string, never parsed by the UI | `jobId`, `sessionId` |
| Timestamp | unix milliseconds UTC, integer | `1762387200000` |
| Size | bytes, integer | `1048576` |

**Deviation from the original design (recorded, not silent).** This document
previously specified `snake_case` DTO fields mapped to camelCase in a TypeScript
adapter. The implemented boundary serializes DTOs with
`#[serde(rename_all = "camelCase")]` instead, so the wire format matches the
TypeScript types one-to-one and there is no mapping layer to drift. Commands and
error codes stay `snake_case`. Rationale and consequences:
[`docs/DECISIONS.md`](docs/DECISIONS.md) (ADR-011).

Rules:

- One command per intent. No `execute(action, payload)` mega-command.
- Long-running work returns an ID immediately; progress arrives by event.
- Every command is authorized by capabilities and by explicit input checks.
- Commands never return partially initialized objects with hidden defaults.

## 2. Result envelope (implemented)

Implemented in `src-tauri/src/ipc.rs` and `src-tauri/src/error.rs`.

```rust
#[derive(Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum CommandResult<T> {
    Ok { data: T },
    Err { error: AppError },
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub code: ErrorCode,        // stable, documented
    pub message: String,        // safe for display
    pub detail: Option<String>, // diagnostic only, redacted; omitted when None
    pub retryable: bool,
    pub correlation_id: String, // serialized as `correlationId`
}
```

Wire shapes:

```jsonc
{ "status": "ok", "data": { /* command-specific DTO */ } }
{ "status": "err", "error": { "code": "access_denied", "message": "…", "retryable": true, "correlationId": "18f3c2a1e40-0001" } }
```

The frontend adapter (`src/services/ipc.ts`) converts the envelope into a
discriminated union and throws `IpcError` for failures. Components never see a
raw `unknown` error.

### Error codes (implemented taxonomy)

| Code | Meaning | Retryable |
|---|---|---|
| `invalid_input` | Failed validation (path, size, enum) | No |
| `not_permitted` | Capability or policy denies the action | No |
| `access_denied` | OS denied access to a resource | Yes |
| `not_found` | Target disappeared | Yes |
| `unsupported` | Not available on this Windows version/edition | No |
| `timeout` | Bounded operation exceeded its budget | Yes |
| `busy` | Conflicting job or locked resource | Yes |
| `cancelled` | User or policy cancelled | No |
| `disk_full` | Storage budget exhausted | No |
| `db_error` | Persistence failure | Yes |
| `internal` | Unexpected error; correlation ID required | No |

`retryable` is derived from the code by `ErrorCode::retryable()`; it is part of
the payload so the UI does not re-implement the table.

Full handling rules: [`docs/ERROR_HANDLING.md`](docs/ERROR_HANDLING.md).

## 3. Release 0 commands

> Field names below are shown exactly as they appear on the wire. Formats decided
> in §1 apply: `camelCase` fields, `snake_case` commands and error codes.

### 3.1 `get_app_info` (implemented)

Read-only, takes no arguments, cannot fail today. Exists to prove the renderer →
IPC → Rust path end to end and to power the backend status indicator.

```rust
// src-tauri/src/commands/app.rs
#[tauri::command]
pub fn get_app_info() -> CommandResult<AppInfo>
```

```jsonc
// response data  (src-tauri/src/dto/app_info.rs, DTO_SCHEMA_VERSION = 1)
{
  "appName": "system-scanner",
  "appVersion": "0.1.0",
  "tauriVersion": "2.12.5",
  "os": "windows",
  "arch": "x86_64",
  "debug": true,
  "schemaVersion": 1
}
```

No machine-identifying data. Frontend: `src/services/app.ts` →
`getAppInfo()`; UI: `src/components/BackendStatus.tsx`.

### 3.2 `get_system_snapshot` (implemented)

Read-only, takes no arguments, requires no elevation. Returns OS, CPU, memory,
disks, and network adapters in one bounded call.

```rust
// src-tauri/src/commands/system.rs
#[tauri::command]
pub async fn get_system_snapshot() -> CommandResult<SystemSnapshotDto>
```

The blocking Win32 work runs on a blocking thread, not the async reactor.

```jsonc
// response data  (src-tauri/src/dto/system_snapshot.rs, SNAPSHOT_SCHEMA_VERSION = 1)
{
  "capturedAt": 1762387200000,
  "schemaVersion": 1,
  "os": {
    "name": "Windows 11 Pro",
    "edition": "Professional",
    "version": "10.0.26100",
    "build": "26100",
    "arch": "x64"
  },
  "cpu": { "model": "…", "logicalCores": 16, "physicalCores": 8 },
  "memory": { "totalBytes": 34359738368, "availableBytes": 12884901888 },
  "disks": [
    {
      "mount": "C:\\",
      "kind": "fixed",
      "label": "System",
      "filesystem": "NTFS",
      "totalBytes": 0,
      "freeBytes": 0
    }
  ],
  "network": [
    { "name": "Ethernet", "description": "…", "isUp": true, "addresses": ["192.0.2.10"] }
  ],
  "unavailable": [
    { "field": "disks[1].totalBytes", "reason": "failed" }
  ]
}
```

Rules, implemented:

- **No request payload.** The earlier `include` sketch was dropped: every group
  is cheap to collect once network drives are excluded. There is no untrusted IPC
  input on this command.
- **Unavailable is mandatory.** A group that cannot be read is omitted from the
  payload (not sent as `null` with empty fields) and named in `unavailable`. A
  missing leaf field keeps its group and is named by path, for example
  `cpu.physicalCores` or `disks[0].freeBytes`.
- **Reasons** are stable values: `access_denied`, `unsupported`, `failed`.
- **Missing is never zero.** A size of `0` is reported only if the OS reports an
  actual zero; an unreadable size is omitted with a reason.
- **Network drives are not queried for label or size.** `GetDiskFreeSpaceExW`
  and `GetVolumeInformationW` can block for tens of seconds on a dead mapping, so
  those fields are reported as `unsupported` and the call stays bounded.
- `capturedAt` is unix milliseconds UTC at collection time.

Frontend: `src/services/system.ts` → `getSystemSnapshot()`; UI:
`src/components/SystemOverview.tsx` (loading, ready, error, refresh, per-field
reason display). Tests: `src-tauri/src/domain/system.rs` (fake port),
`src-tauri/src/dto/system_snapshot.rs` (wire shape),
`src-tauri/src/adapters/windows/system_info.rs` (native structural smoke test),
`src/components/SystemOverview.test.tsx`.

### 3.3 `list_processes` (proposed)

```jsonc
// request — bounded
{ "limit": 500, "offset": 0, "sort": { "field": "workingSetBytes", "dir": "desc" } }

// response data
{
  "capturedAt": 1762387200000,
  "total": 312,
  "items": [
    {
      "pid": 4242,
      "parentPid": 1024,
      "identity": "4242:1762380000",
      "name": "explorer.exe",
      "executablePath": "C:\\Windows\\explorer.exe",
      "workingSetBytes": 104857600,
      "accessLimited": false
    }
  ]
}
```

`limit` is capped server-side (proposed max 1000). `accessLimited` marks rows
whose metadata could not be fully read.

### 3.4 `get_diagnostics` (proposed)

Returns app version, OS build, log location, recent redacted log lines, and
feature availability. Used to build a shareable support bundle.

### 3.5 Events (proposed, none emitted today)

Proposed events for Release 0:

| Event | Payload |
|---|---|
| `system:refreshed` | `{ capturedAt, durationMs, partial }` |
| `system:refresh_failed` | `{ error }` |
| `log:record` | redacted structured record (opt-in stream) |

## 4. Proposed commands — Release 1 (storage)

| Command | Purpose | Returns |
|---|---|---|
| `start_scan` | Begin a scan | `{ jobId, sessionId }` |
| `cancel_scan` | Cooperative cancel | `{ accepted: bool }` |
| `get_scan_status` | Authoritative job state | status DTO |
| `get_scan_summary` | Aggregates only | counts, sizes, top types |
| `query_files` | Paged, filtered, sorted list | page + total |
| `start_hashing` | Optional bounded enrichment | `{ job_id }` |
| `get_duplicate_groups` | Groups by size + fingerprint/hash | paged groups |
| `delete_scan` | Remove a session's records | `{ deleted: bool }` |

Key limits:

| Limit | Proposed value |
|---|---|
| `query_files.limit` | max 500 |
| Concurrent scans | 1 (Release 1) |
| Concurrent hashes | ≤ 2, size-capped |
| Event rate | ≤ 10/s per job, coalesced |
| Path length | validated against Windows limits |

### Scan events

| Event | Payload |
|---|---|
| `scan:progress` | `{ jobId, seq, phase, filesSeen, bytesSeen, skipped, elapsedMs }` |
| `scan:completed` | `{ jobId, summary, partial }` |
| `scan:failed` | `{ jobId, error }` |
| `scan:cancelled` | `{ jobId, reason, elapsedMs }` |

`seq` is monotonic per job; the UI discards out-of-order or stale updates.

## 5. Proposed commands — privileged (Release 2/3, gated)

These must not be implemented until the threat model, audit trail, confirmation
flow, and recovery path exist.

| Command | Preconditions |
|---|---|
| `quarantine_file` | Hash verified, target confirmed, protected-path policy passed, audit row written |
| `restore_quarantine_item` | Integrity verified, destination free, audit row written |
| `terminate_process` | Explicit target identity (PID + start time), confirmation, audit row |
| `stop_service` | Explicit service name, confirmation, audit row |

Every privileged command requires a `confirmation` object echoing the exact
target and consequence the user approved. Without it: `invalid_input`.

## 6. Authorization model (proposed)

- Tauri capabilities gate what the webview may invoke.
- The Rust command layer re-checks policy; capability configuration is not the
  only control.
- Read-only commands cannot reach privileged code paths (separate modules).
- Privileged commands are never available while a scan is writing to the DB in a
  way that would leave inconsistent state.

See [`docs/ACCESS_CONTROL.md`](docs/ACCESS_CONTROL.md).

## 7. Versioning and compatibility

- Every DTO module exports a `schema_version` constant, and every DTO that can be
  observed by the UI carries it (`AppInfo.schemaVersion = 1` today).
- Additive fields are safe; renames/removals require a version bump and a UI
  adapter update in the same change.
- IPC DTOs and persisted rows evolve independently; a migration maps old rows.
- The UI and Rust core ship together in one package, but a stale webview must
  still fail predictably (show "reload required" rather than wrong data).

## 8. Frontend contract (implemented)

`src/services/ipc.ts` is the **only** module that calls `invoke`:

```ts
export async function invokeCommand<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  const result = await invoke<CommandResult<T>>(command, args);
  if (result.status === "err") throw new IpcError(result.error);
  return result.data;
}
```

Domain modules wrap it (`src/services/app.ts` → `getAppInfo()`), and components
call the domain module. Implemented rules for `src/services`:

- One module per domain; no component calls `invoke` directly.
- The envelope is unwrapped once, so callers never handle `status` or `unknown`.
- `IpcError` carries `code`, `retryable`, and `correlationId` for UI decisions and
  support reports.

Not yet done: runtime validation of response payloads (no `zod` or hand-written
guards), so a contract drift would currently surface as `undefined` at render
time rather than as a loud failure. Tracked in `docs/BACKLOG.md` (B-090).

## 9. Testing the contract

Required before any command ships:

1. Unit test for validation (valid, invalid, boundary, oversized input).
2. Test that malformed payloads produce `INVALID_INPUT`, not a panic.
3. Test authorization: read-only capability cannot invoke a privileged command.
4. UI adapter test with a mocked IPC layer for success and each error class.
5. Cancellation/timeout test for long-running commands.

Status today:

| Check | Status |
|---|---|
| Envelope serialization (ok/err, camelCase, omitted `detail`) | ✅ `src-tauri/src/ipc.rs` tests |
| Error code taxonomy + unique correlation IDs | ✅ `src-tauri/src/error.rs` tests |
| DTO wire shape + `schemaVersion` | ✅ `src-tauri/src/dto/app_info.rs`, `system_snapshot.rs` tests |
| Adapter unwrap / `IpcError` / argument pass-through | ✅ `src/services/ipc.test.ts` |
| UI states (loading, ready, error, retry) | ✅ `BackendStatus.test.tsx`, `SystemOverview.test.tsx` |
| Unavailable-field reporting (group and leaf) | ✅ domain + DTO tests; native smoke test |
| Native adapter structural invariants | ✅ `adapters/windows/system_info.rs` (Windows-only test) |
| Input validation, authorization, cancellation | ❌ `get_system_snapshot` takes no input; no long-running command yet |

See [`TESTING.md`](TESTING.md) and [`AI_TESTING_RULES.md`](AI_TESTING_RULES.md).

## 10. Related

- [`docs/SYSTEM_DESIGN.md`](docs/SYSTEM_DESIGN.md) — job lifecycle and sequencing
- [`docs/ERROR_HANDLING.md`](docs/ERROR_HANDLING.md) — failure semantics
- [`docs/PERFORMANCE.md`](docs/PERFORMANCE.md) — IPC budgets
