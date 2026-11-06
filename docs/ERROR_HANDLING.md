# Error Handling

> **Status:** No error model is implemented. `lib.rs` contains a single `expect()`
> on startup. Everything here is the target design.
> **Last updated:** 2026-11-06

---

## 1. Goals

1. **Actionable** — the user knows what happened and what to do next.
2. **Honest** — partial results are reported as partial; unavailable is not zero.
3. **Safe** — no internal paths, stack traces, or secrets in user-facing messages.
4. **Stable** — error codes are part of the contract and do not change casually.
5. **Diagnosable** — one correlation ID links UI message, log entry, and job.

## 2. Taxonomy

| Code | Category | Retryable | Typical cause | User message strategy |
|---|---|---|---|---|
| `INVALID_INPUT` | Client | No | Bad path, out-of-range limit | State the constraint |
| `NOT_PERMITTED` | Policy | No | Denied by capability/policy | "Not allowed for this operation" |
| `ACCESS_DENIED` | OS | Sometimes | ACL, protected process | Explain elevation/permission |
| `NOT_FOUND` | OS | Sometimes | File/process disappeared | Offer refresh |
| `UNSUPPORTED` | Platform | No | API unavailable on this build | Explain limitation |
| `TIMEOUT` | Transient | Yes | Slow device/WMI hang | Offer retry with reduced scope |
| `BUSY` | Transient | Yes | Job already running, DB locked | Show existing job / wait |
| `CANCELLED` | User | No | User or policy cancelled | Informational, not an error dialog |
| `DISK_FULL` | Resource | After cleanup | DB/quarantine full | Show what to free |
| `DB_ERROR` | Persistence | Sometimes | Corrupt/locked/migration | Recovery instructions |
| `INTERNAL` | Unknown | No | Bug | Apology + correlation ID |

Codes live in one enum in Rust and one union in TypeScript. They must not be
improvised per screen.

## 3. Layered model

```text
Adapter      PlatformError   (HRESULT, io::Error, WMI failure)
   │ map
Domain       DomainError     (policy violations, invalid state)
   │ map
Command      AppError code   (stable code + safe message + retryable)
   │ serialize
UI           IpcError        (typed union, correlation_id)
   │ render
Component    user message + next action
```

Rules:

- Raw platform errors never cross IPC.
- Domain errors never include a path unless redaction is applied.
- Commands map errors exactly once, in one place.
- The UI never string-matches messages; it switches on `code`.

## 4. Rust implementation (proposed)

```rust
#[derive(Debug, thiserror::Error)]
pub enum CollectorError {
    #[error("access denied")]
    AccessDenied,
    #[error("not supported on this platform")]
    Unsupported,
    #[error("target disappeared")]
    NotFound,
    #[error("timed out after {0:?}")]
    Timeout(std::time::Duration),
    #[error("os error: {0}")]
    Os(#[from] std::io::Error),
}

impl From<CollectorError> for AppError {
    fn from(e: CollectorError) -> Self {
        match e {
            CollectorError::AccessDenied => AppError::new(Code::AccessDenied, true),
            CollectorError::Unsupported => AppError::new(Code::Unsupported, false),
            CollectorError::NotFound    => AppError::new(Code::NotFound, true),
            CollectorError::Timeout(_)  => AppError::new(Code::Timeout, true),
            CollectorError::Os(_)       => AppError::new(Code::Internal, false),
        }
        .with_correlation_id(new_id())
    }
}
```

## 5. Partial results

A scan or collection is a **success with caveats**, not a failure, when some items
could not be read.

```jsonc
{
  "status": "ok",
  "data": {
    "summary": { "files_seen": 100000, "denied": 120, "skipped": 8 },
    "partial": true,
    "partial_reasons": [
      { "reason": "access_denied", "count": 120 },
      { "reason": "unsupported",  "count": 8 }
    ]
  }
}
```

UI rules:

- Show "partial" prominently; never present partial counts as complete.
- The summary must be reachable without opening a modal.
- An empty table must say *why* it is empty (denied, filtered, not scanned).

## 6. Retry policy

| Operation | Retry |
|---|---|
| Read-only collector | 1 retry with backoff for transient codes only |
| DB write | Bounded retry (3) on `BUSY`, then fail the batch |
| Hashing | No automatic retry; mark `failed` with reason |
| Privileged action | **Never** auto-retry; require a new confirmation |

Backoff: 100 ms, 300 ms, 900 ms (cap), jittered. Never retry `INVALID_INPUT`,
`NOT_PERMITTED`, or `CANCELLED`.

## 7. Cancellation

- `CANCELLED` is not an error state in the UI; it is a normal outcome.
- Cancellation must be observable within the documented latency budget.
- Resources (handles, transactions) are released before the state becomes
  `cancelled`.
- Partial data may be retained and must be labeled.

## 8. Panics

`panic = "abort"` in release. Therefore:

- Panics are **bugs**, not a control-flow mechanism.
- No `unwrap()`/`expect()` on input in command or worker paths.
- The only accepted `expect()` is "cannot construct the app" at startup, which
  should become a user-visible fatal-error dialog.
- Next launch reconciles interrupted sessions.

## 9. Logging and correlation

- Every command generates a `correlation_id`.
- Log at `error` with code + correlation ID + safe context.
- UI displays the correlation ID in a copyable form for support.
- Never log the raw error chain if it contains paths or user data.

## 10. User-facing message guidelines

| Do | Don't |
|---|---|
| "Access denied to 3 folders. Run as administrator to include them." | "Error 0x80070005" |
| "Scan stopped. 42% was analyzed; results are partial." | "Failed." |
| "Could not read the disk. It may be disconnected." | "I/O error" |
| "This Windows version does not expose service start types." | "Unsupported" |

Rules:

- Name the affected object when it is safe to do so.
- State whether the user can retry and what changes the outcome.
- Never blame the user for an OS restriction.
- Never imply a security verdict from a collection error.

## 11. Testing requirements

- One test per error code mapping.
- Malformed IPC payload → `INVALID_INPUT`, never a panic.
- Adapter access-denied → `ACCESS_DENIED` with partial result, not an empty list.
- Cancellation at each phase → `CANCELLED` within budget.
- Retry exhaustion → terminal error with the original code preserved.
- Message redaction test: no absolute path or username in the DTO.

## 12. Related

- [`../API.md`](../API.md) §2 — error envelope
- [`FAILURE_MODES.md`](FAILURE_MODES.md) — scenario matrix
- [`MONITORING.md`](MONITORING.md) — logging
