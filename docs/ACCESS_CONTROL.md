# Access Control

> **Status:** No authorization system is implemented. Today the app runs as the
> invoking user with `core:default` Tauri capabilities only.
> **Last updated:** 2026-11-06

---

## 1. Model

This is a **single-user desktop application**, not a multi-tenant service.
Classic RBAC/ABAC (roles, policies, subjects) does not apply in the usual sense.
Access control here means:

1. **OS-level:** what the logged-in Windows user is allowed to read/change.
2. **Application-level:** which IPC commands the renderer may invoke.
3. **Policy-level:** which operations the product permits at all (protected paths,
   destructive actions).
4. **Confirmation-level:** what the user must explicitly approve.

No custom user accounts, no login, no roles. Windows identity is the identity.

## 2. Layer 1 — Windows user context

| Aspect | Behavior |
|---|---|
| Default integrity | As invoked; no `requireAdministrator` manifest |
| Elevation | Per-operation only, opt-in, with a reason; fails safely if denied |
| Data access | Only what the user's ACLs permit |
| Protected processes | Reported as access-limited, never bypassed |
| Multi-user | Not supported; data lives in the current user's profile |

Rules:

- Never request elevation at startup.
- Never retry elevation silently.
- A read-only feature must work for a standard user.
- If an operation requires admin, explain why *before* prompting.

## 3. Layer 2 — Tauri capabilities

Current:

```json
{ "identifier": "default", "windows": ["main"], "permissions": ["core:default"] }
```

Policy:

- One capability file per window; no wildcard windows.
- Add a permission only when an implemented feature requires it.
- No `fs:*`, `shell:*`, `http:*`, or plugin permissions until a reviewed feature
  needs them (and likely never for `shell`/`http`).
- Capability changes are security changes: review + document + test.

Capability is necessary but **not sufficient**: the Rust command layer re-checks
policy. This defends against a capability misconfiguration.

## 4. Layer 3 — Operation classes

| Class | Examples | Who may invoke | Requirements |
|---|---|---|---|
| Read-only collection | system snapshot, process list | Any standard user | None beyond capability |
| Read-only privileged data | some service/process metadata | Standard user, partial | Report access-limited fields |
| Local persistence | save session, read history | Any user | Data in the user's profile |
| Reversible system change | quarantine, restore | User, explicit confirmation | Audit + integrity + recovery |
| Irreversible system change | permanent delete | User, explicit confirmation | Audit + backup + strongest warning |
| Privileged change | stop service, terminate protected process | Elevated user, explicit confirmation | Elevation + audit |

The filesystem traversal is read-only collection. Since B-025 it **is reachable
over IPC** through `start_scan`, which accepts caller-supplied roots. No new
Tauri capability or plugin was added; the command is part of the app's own
handler set. Roots are validated in Rust (1–64, non-empty, no NUL, length cap,
absolute) and the scan stays a standard-user operation. Directories the user
cannot read are reported as `denied`, never bypassed, and the response carries
only aggregate counters, never paths.

Device-namespace roots are rejected in Rust (`dto/scan.rs`): `\\.\...`, `\??\...`,
and any `\\?\` path that is not a drive (`\\?\C:\`) or UNC (`\\?\UNC\...`) form,
including `\\?\GLOBALROOT\...` and `\\?\Volume{...}\`. Forward slashes are
normalized first.

**Remaining gap:** arbitrary UNC shares and any other absolute path are still
accepted and walked read-only (reparse points are never followed). Whether to
restrict roots further, for example with an allow-list tied to a folder picker or
a block on network paths, is a product decision for the B-040 threat-model
review. Reserved DOS device names inside a normal path (`C:\dir\CON`) and the
ACL of the root are not checked beyond what the OS returns.

## 5. Layer 4 — Protected paths

Proposed default-deny list (to be finalized before any write feature):

```text
C:\Windows\**
C:\Program Files\**
C:\Program Files (x86)\**
C:\ProgramData\Microsoft\**
%SystemRoot%\System32\**
%USERPROFILE%\AppData\Local\Microsoft\**   (selective)
the application's own install and data directories (except its own store)
```

Rules:

- Deny by default; allow only through an explicit, reviewed policy.
- Path comparison uses canonical, case-insensitive Windows semantics, including
  short (8.3) names and reparse resolution.
- Policy is enforced in Rust, never in the UI.
- A denial is reported with a reason, not a silent skip.

## 6. Layer 5 — Confirmation

Every state-changing action requires a confirmation object that echoes:

```jsonc
{
  "action": "quarantine",
  "target": "C:\\Users\\me\\Downloads\\file.exe",
  "consequence": "The file will be moved to quarantine and cannot run from its current location.",
  "reversible": true,
  "recovery": "Restore from Quarantine at any time.",
  "nonce": "…"
}
```

Rules:

- The user sees the exact target and consequence.
- No pre-checked destructive default.
- The Rust side verifies the nonce/confirmation matches the action; a stale
  approval cannot be replayed.
- Cancellation is always available.

See [`THREAT_MODEL.md`](THREAT_MODEL.md) T-05.

## 7. Future: multi-user or managed deployment

Explicitly out of scope. If it is ever proposed, it requires:

- a real identity model and role definitions;
- server-side policy if a fleet console exists;
- audit export and retention;
- a new threat model and privacy review;
- a separate architecture (ADR).

Do not add role/permission abstractions preemptively.

## 8. Authorization tests (required)

```text
[ ] Read-only command cannot invoke a privileged code path
[ ] A capability not granted cannot be invoked
[ ] Invalid/stale confirmation nonce is rejected
[ ] Protected path is denied with a reason
[ ] Standard user can complete all read-only flows
[ ] Elevation denial produces a recoverable error, not a partial state change
[ ] IPC caller cannot supply a path that escapes the selected root
```

## 9. Open questions

1. Which specific operations genuinely require elevation?
2. Is a machine-wide policy override ever needed (enterprise)?
3. Should quarantine live in the user profile or a machine-wide protected folder?
4. How is the protected-path list made auditable and updatable?
