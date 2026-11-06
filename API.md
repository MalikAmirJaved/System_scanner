# API

> **Status:** **Interface not implemented.** This repository exposes **no IPC
> commands and no HTTP API** today. `src-tauri/src/lib.rs` registers no handlers.
> Everything below is a **proposed contract** to be implemented in Release 0/1.
>
> **Last updated:** 2026-11-06 · **Related:** [`ARCHITECTURE.md`](ARCHITECTURE.md)

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
| DTO field | `snake_case` on the wire, mapped to camelCase in TS adapters | `started_at` |
| ID | opaque string, never parsed by the UI | `job_id`, `session_id` |
| Timestamp | unix milliseconds UTC, integer | `1762387200000` |
| Size | bytes, integer | `1048576` |

Rules:

- One command per intent. No `execute(action, payload)` mega-command.
- Long-running work returns an ID immediately; progress arrives by event.
- Every command is authorized by capabilities and by explicit input checks.
- Commands never return partially initialized objects with hidden defaults.

## 2. Result envelope (proposed)

```rust
#[derive(Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum CommandResult<T> {
    Ok { data: T },
    Err { error: AppError },
}

#[derive(Serialize)]
pub struct AppError {
    pub code: ErrorCode,       // stable, documented
    pub message: String,       // safe for display
    pub detail: Option<String>,// diagnostic only, redacted
    pub retryable: bool,
    pub correlation_id: String,
}
```

The frontend adapter converts this into a discriminated union. Components never
see a raw `unknown` error.

### Error codes (proposed taxonomy)

| Code | Meaning | Retryable |
|---|---|---|
| `INVALID_INPUT` | Failed validation (path, size, enum) | No |
| `NOT_PERMITTED` | Capability or policy denies the action | No |
| `ACCESS_DENIED` | OS denied access to a resource | Sometimes |
| `NOT_FOUND` | Target disappeared | Sometimes |
| `UNSUPPORTED` | Not available on this Windows version/edition | No |
| `TIMEOUT` | Bounded operation exceeded its budget | Yes |
| `BUSY` | Conflicting job or locked resource | Yes |
| `CANCELLED` | User or policy cancelled | No |
| `DISK_FULL` | Storage budget exhausted | After cleanup |
| `DB_ERROR` | Persistence failure | Sometimes |
| `INTERNAL` | Unexpected error; correlation ID required | No |

Full handling rules: [`docs/ERROR_HANDLING.md`](docs/ERROR_HANDLING.md).

## 3. Proposed commands — Release 0

### 3.1 `get_system_snapshot`

Read-only. Returns OS, CPU, memory, disks, and adapters.

```jsonc
// request
{ "include": ["os", "cpu", "memory", "disks", "network"] }

// response data
{
  "captured_at": 1762387200000,
  "os":    { "name": "Windows 11 Pro", "version": "10.0.26100", "arch": "x86_64" },
  "cpu":   { "model": "…", "logical_cores": 16, "physical_cores": 8 },
  "memory":{ "total_bytes": 34359738368, "available_bytes": 12884901888 },
  "disks": [ { "mount": "C:\\", "total_bytes": 0, "free_bytes": 0, "kind": "fixed" } ],
  "network": [ { "name": "Ethernet", "status": "up", "addresses": ["192.0.2.10"] } ],
  "unavailable": [ { "field": "cpu.model", "reason": "access_denied" } ]
}
```

`unavailable` is mandatory when any field could not be collected. A missing field
must never be represented as `0`, `""`, or `null` without a reason.

### 3.2 `list_processes`

```jsonc
// request — bounded
{ "limit": 500, "offset": 0, "sort": { "field": "working_set_bytes", "dir": "desc" } }

// response data
{
  "captured_at": 1762387200000,
  "total": 312,
  "items": [
    {
      "pid": 4242,
      "parent_pid": 1024,
      "identity": "4242:1762380000",
      "name": "explorer.exe",
      "executable_path": "C:\\Windows\\explorer.exe",
      "working_set_bytes": 104857600,
      "access_limited": false
    }
  ]
}
```

`limit` is capped server-side (proposed max 1000). `access_limited` marks rows
whose metadata could not be fully read.

### 3.3 `get_diagnostics`

Returns app version, OS build, log location, recent redacted log lines, and
feature availability. Used to build a shareable support bundle.

### 3.4 `refresh_status` (event-driven)

Proposed events for Release 0:

| Event | Payload |
|---|---|
| `system:refreshed` | `{ captured_at, duration_ms, partial }` |
| `system:refresh_failed` | `{ error }` |
| `log:record` | redacted structured record (opt-in stream) |

## 4. Proposed commands — Release 1 (storage)

| Command | Purpose | Returns |
|---|---|---|
| `start_scan` | Begin a scan | `{ job_id, session_id }` |
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
| `scan:progress` | `{ job_id, seq, phase, files_seen, bytes_seen, skipped, elapsed_ms }` |
| `scan:completed` | `{ job_id, summary, partial }` |
| `scan:failed` | `{ job_id, error }` |
| `scan:cancelled` | `{ job_id, reason, elapsed_ms }` |

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
target and consequence the user approved. Without it: `INVALID_INPUT`.

## 6. Authorization model (proposed)

- Tauri capabilities gate what the webview may invoke.
- The Rust command layer re-checks policy; capability configuration is not the
  only control.
- Read-only commands cannot reach privileged code paths (separate modules).
- Privileged commands are never available while a scan is writing to the DB in a
  way that would leave inconsistent state.

See [`docs/ACCESS_CONTROL.md`](docs/ACCESS_CONTROL.md).

## 7. Versioning and compatibility

- DTOs carry `schema_version` once persisted or exported.
- Additive fields are safe; renames/removals require a version bump and a UI
  adapter update in the same change.
- IPC DTOs and persisted rows evolve independently; a migration maps old rows.
- The UI and Rust core ship together in one package, but a stale webview must
  still fail predictably (show "reload required" rather than wrong data).

## 8. Frontend contract

```ts
// src/services/system.ts  (proposed)
import { invoke } from "@tauri-apps/api/core";
import type { SystemSnapshot } from "./dto";

export async function getSystemSnapshot(): Promise<SystemSnapshot> {
  const res = await invoke<CommandResult<SystemSnapshot>>("get_system_snapshot", {
    include: ["os", "cpu", "memory", "disks", "network"],
  });
  if (res.status === "err") throw new IpcError(res.error);
  return res.data;
}
```

Rules for `src/services`:

- One module per domain; no component calls `invoke` directly.
- Runtime-validate responses in development (`zod` or hand-written guards) so a
  contract mismatch fails loudly instead of rendering `undefined`.
- Own event subscription/unsubscription and cleanup.

## 9. Testing the contract

Required before any command ships:

1. Unit test for validation (valid, invalid, boundary, oversized input).
2. Test that malformed payloads produce `INVALID_INPUT`, not a panic.
3. Test authorization: read-only capability cannot invoke a privileged command.
4. UI adapter test with a mocked IPC layer for success and each error class.
5. Cancellation/timeout test for long-running commands.

See [`TESTING.md`](TESTING.md) and [`AI_TESTING_RULES.md`](AI_TESTING_RULES.md).

## 10. Related

- [`docs/SYSTEM_DESIGN.md`](docs/SYSTEM_DESIGN.md) — job lifecycle and sequencing
- [`docs/ERROR_HANDLING.md`](docs/ERROR_HANDLING.md) — failure semantics
- [`docs/PERFORMANCE.md`](docs/PERFORMANCE.md) — IPC budgets
