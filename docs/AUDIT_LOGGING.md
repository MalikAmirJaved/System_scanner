# Audit Logging

> **Status:** Not implemented. No audit record, table, or log exists.
> **Last updated:** 2026-11-06

---

## 1. Purpose

Audit logging answers, for every state-changing action: **who did what, to which
target, when, with what result, and after what confirmation.**

It exists for the user (to understand and undo), for support (to diagnose), and for
safety (to make silent actions impossible). It is **not** surveillance: it stays
local and contains no file contents.

## 2. Scope

| Action | Audited |
|---|---|
| File quarantined / restored / purged | Yes |
| Process terminated | Yes |
| Service started/stopped | Yes |
| Scan started / cancelled / completed | Yes (session record) |
| Settings change affecting privacy/privilege/resource | Yes |
| Elevation requested / denied | Yes |
| Audit log exported / cleared | Yes |
| Read-only view of system data | No (too noisy, no state change) |

## 3. Record schema (proposed)

```jsonc
{
  "id": "01J...",
  "ts": "2026-11-06T12:00:00.123Z",
  "action": "quarantine",
  "target": {
    "path": "C:\\Users\\me\\Downloads\\file.exe",
    "size_bytes": 1048576,
    "sha256": "…"
  },
  "confirmation": {
    "shown": "Move to quarantine; restore possible",
    "nonce": "…",
    "user_approved": true
  },
  "result": "succeeded",
  "error_code": null,
  "elevated": false,
  "actor": "local-user",
  "app_version": "0.1.0",
  "prev_hash": "…",
  "entry_hash": "…"
}
```

Rules:

- `action`, `target`, `confirmation`, `result`, `ts` are mandatory.
- A denied or failed attempt is recorded, not only successes.
- No file contents, command lines, or credentials.
- Paths are stored verbatim locally (needed for restore) but redacted on export.

## 4. Storage

Proposed table `action_history` (see [`DATABASE.md`](../DATABASE.md) §5.8):

- Append-only: no update except attaching `completed_at`/`result` for an in-flight
  action.
- `ON DELETE` never cascades from sessions.
- Retention: until uninstall, or until no quarantined item depends on the record.
- ACL: current user only.

## 5. Tamper evidence

Single-user local storage cannot be made tamper-proof against an administrator.
The goal is **tamper-evident**:

```text
entry_hash = SHA-256(prev_hash || canonical_json(entry_without_hash))
```

- The first entry uses a fixed genesis value.
- Verification walks the chain and reports the first broken link.
- A broken chain is surfaced in diagnostics as "audit integrity unknown", never
  silently ignored.
- Export includes the chain so a support engineer can verify it.

Open question: is this warranted for v1, or is append-only sufficient? See
[`DECISIONS.md`](DECISIONS.md) and [`THREAT_MODEL.md`](THREAT_MODEL.md) T-08.

## 6. What must never be logged

- File contents or previews.
- Full command lines or environment variables.
- Credentials, tokens, keys.
- Personal data unrelated to the action.
- Raw IPC payloads.

## 7. User-facing view

Proposed "History" screen:

```text
[time] Quarantined  file.exe (1.0 MB)      Restored  [Restore]
[time] Scan completed  1,204,332 files     Partial: 12 denied
[time] Terminate process  updater.exe (PID 4242)   Failed: access denied
```

- Filterable by action and date.
- Explains what is reversible.
- Export with redaction.

## 8. Correlation with logs

- Each audit row has an `id`; diagnostic logs reference it.
- Logs may be rotated and deleted; audit rows are retained.
- Audit does not replace logs: logs are verbose and short-lived, audit is minimal
  and durable.

## 9. Failure behavior

| Failure | Behavior |
|---|---|
| Audit write fails before action | **Do not perform the action** (fail closed) |
| Audit write fails after action | Surface a warning; record a compensating entry if possible |
| Chain broken | Mark integrity unknown, warn in diagnostics |
| Disk full | Block further state-changing actions, explain why |

## 10. Testing requirements

- Every action writes exactly one audit row, including denials.
- A failed action records the error code and leaves no false success.
- Export redacts paths/usernames.
- Chain verification detects a modified/removed row.
- Audit retention never deletes a row needed to restore a quarantined file.

## 11. Open questions

1. Mandatory hash chain, or optional?
2. Should read-only scans be audited (they create session records already)?
3. What is the retention when quarantine is empty?
4. Is audit export part of the support bundle or a separate action?
