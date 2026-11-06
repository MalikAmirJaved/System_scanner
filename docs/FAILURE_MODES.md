# Failure Modes & Effects

> **Status:** Design reference. Most failures below cannot occur yet because the
> corresponding feature does not exist. **Last updated:** 2026-11-06

---

## 1. How to use this document

For each component, know:

1. **How it fails** — the realistic failure signals.
2. **What the user sees** — honest, actionable.
3. **What the system does** — continue, degrade, or abort.
4. **How it recovers** — automatic, user action, or documented limitation.

## 2. Startup and shell

| Failure | Detection | Handling | User sees |
|---|---|---|---|
| WebView2 missing | Tauri init error | Fatal dialog with install link | "WebView2 is required" |
| Data dir not writable | Write probe at startup | Memory-only mode + banner | "History will not be saved" |
| DB corrupt | `quick_check` | Preserve, fresh start, offer restore | Recovery banner |
| DB newer than app | `user_version` | Refuse to modify | "This data was created by a newer version" |
| Migration failure | Transaction error | Roll back, keep old schema | Failing migration ID + retry |
| Panic at startup | Process abort | Next launch reports previous crash | Fatal screen |
| Window fails to create | Tauri error | Exit with logged reason | Nothing (log only) — **gap** |

## 3. System collection

| Failure | Handling |
|---|---|
| API unavailable on this Windows build | Mark field `unsupported`; snapshot is partial |
| Access denied | Mark `denied`; do not substitute zero |
| WMI service stopped | Empty result + reason; do not block |
| WMI hang | Timeout, cancel, partial |
| CPU/memory counter returns garbage | Validate range; discard and mark unavailable |
| Network adapter list empty | Empty is valid; distinguish from failure |
| Device removed mid-collection | `disappeared`; continue |

Rule: a partial snapshot is **success with warnings**, never a silent zero.

## 4. Process list

| Failure | Handling |
|---|---|
| Access denied on a process | Include PID/name; `access_limited = true` |
| Process exits during enumeration | Skip or mark `disappeared`; no error |
| PID reused | Identity uses PID + start time; never match on PID alone |
| Protected process | Report limited metadata; never attempt to bypass |
| Enumeration exceeds limit | Return page + total; do not truncate silently |

## 5. Storage scan

| Failure | Handling |
|---|---|
| Directory access denied | Count `denied`, continue |
| File disappears | Outcome `disappeared`, continue |
| Reparse loop | Visited set + depth cap, stop that branch |
| Path too long | Report `unsupported`/`failed`, continue |
| Disk disconnects | Abort that root, keep partial results |
| Disk full (DB) | Stop writes, mark session failed, keep prior data |
| Cancellation | Stop within latency budget, keep partial, label it |
| Worker panic | Job fails; app may abort (`panic = abort`) — a test gap |
| Hash read error | `hash_status = failed`, continue |
| Hashing exceeds size cap | `skipped`, explicitly reported |
| Metadata changes mid-scan | Record last observed; label scan approximate |
| Millions of entries | Backpressure slows traversal; memory stays flat |

## 6. Persistence

| Failure | Handling |
|---|---|
| DB locked | `busy_timeout`, bounded retry, then `BUSY` |
| Disk full | Abort batch, mark job failed, keep previous history |
| Corrupt page | Preserve file, recovery path |
| Interrupted write | Transaction rollback |
| Interrupted scan | Session `interrupted` at next start |
| Schema mismatch | Refuse, explain, offer backup restore |
| Backup verification fails | Delete suspect backup, keep source |

## 7. IPC and UI

| Failure | Handling |
|---|---|
| Invalid payload | `INVALID_INPUT`, no panic |
| Oversized payload | Reject before allocation |
| Unknown command | Reject; log at debug |
| Unauthorized command | `NOT_PERMITTED`; audit if privileged |
| Duplicate start | `BUSY` + existing job ID |
| Event out of order | UI drops by `seq` |
| Stale response overwrites newer state | Request generation/sequence check |
| Command hangs | UI timeout + cancel; Rust job cancelled |
| Webview reload during job | Reconcile via `get_scan_status` |
| App closes during job | Cancel + flush; mark cancelled/interrupted |

## 8. Privileged and destructive operations

| Failure | Handling |
|---|---|
| Elevation denied | Abort cleanly, no partial change, explain |
| Target replaced (TOCTOU) | Re-verify identity, abort |
| Target locked | Report, do not force |
| Quarantine disk full | Abort before removing source |
| Restore destination exists | Do not overwrite; offer rename/cancel |
| Restore hash mismatch | Refuse, keep quarantined, report |
| Audit write fails | Refuse to perform the action |
| Process exits before termination | Report "already exited", no error dialog |
| Service control denied | Report code, no retry loop |

## 9. External and environment

| Failure | Handling |
|---|---|
| Network disabled | No effect on baseline; opt-in features show unavailable |
| Antivirus blocks the app | Explain false positive; provide signing/exclusion guidance |
| Corporate policy blocks install | Document MSI/enterprise needs |
| Clock skew | Store UTC; do not trust wall clock for ordering; use sequence numbers |
| Locale differences | Do not parse localized OS strings; use APIs |
| Non-ASCII/long paths | Tested fixtures; no lossy conversions |

## 10. Severity of user impact

| Impact | Examples |
|---|---|
| **Blocking** | App cannot start, DB unrecoverable, data loss |
| **Degraded** | Partial snapshot, denied paths, no history persistence |
| **Cosmetic** | Missing optional field, slow refresh |

Every blocking failure must have a documented recovery path in
[`RECOVERY_PLAN.md`](RECOVERY_PLAN.md).

## 11. Test obligations

Each row above that becomes implemented needs a test proving the handling, not
just the happy path. See [`../AI_TESTING_RULES.md`](../AI_TESTING_RULES.md).
