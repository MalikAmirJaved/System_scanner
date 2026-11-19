# Access Control

> **Status:** No authorization system is implemented. Today the app runs as the
> invoking user with `core:default` Tauri capabilities only.
> **Last updated:** 2026-11-19

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
| Read-only collection | system snapshot, process list, startup/service inventory | Any standard user | None beyond capability |
| Read-only privileged data | some service/process metadata | Standard user, partial | Report access-limited fields |
| Local persistence | save session, read history | Any user | Data in the user's profile |
| Reversible system change | quarantine, restore | User, explicit confirmation | Audit + integrity + recovery |
| Irreversible system change | permanent delete | User, explicit confirmation | Audit + backup + strongest warning |
| Privileged change | stop service, terminate protected process | Elevated user, explicit confirmation | Elevation + audit |

The startup/service inventory (`get_startup_inventory`, B-043) is read-only
collection and needs no elevation: the SCM enumeration and the `Run`/`RunOnce`
keys are readable by a standard user, and anything refused is reported through
`unavailable` instead of being retried with more privilege (ADR-028). There is
no command to change a service or a startup entry.

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

**Window closed (2026-11-19, B-041).** The B-040 review (2026-11-17) recorded one
concrete window: `visit_directory` re-checked the reparse attribute with
`symlink_metadata` and then called `read_dir` **by path**, so a directory replaced
by a junction in between was enumerated even though the check passed
(`docs/THREAT_MODEL.md` §11.3 F-01). Directories are now opened once without
following their final link, identified by volume serial + 128-bit file id, and
enumerated through that same handle (`adapters/filesystem/directory.rs`); a
mismatch is reported as `disappeared`, and a volume without file ids is
unverifiable (KI-056). Whether to restrict roots further — an allow-list tied to a folder picker,
a block on network shares, a warning for removable media — is still an **open
product decision**; arbitrary UNC shares and other absolute paths remain accepted
and walked read-only today. Reserved DOS device names inside a normal path
(`C:\dir\CON`) and the ACL of the root are not checked beyond what the OS returns.

## 5. Layer 4 — Protected paths

> **Status:** Implemented as a module, **not enforced** by any command
> (2026-11-19, B-041). Nothing in the product changes state, so nothing calls it
> yet. See KI-057.

`domain/policy.rs` holds the policy (`ProtectedPathPolicy`), with no I/O and no
platform import. `default_roots(...)` takes the machine-specific directories as
arguments, so tests describe a machine instead of inheriting one.

Protected roots and the reason each denial reports:

| Root | Reason code |
|---|---|
| `%SystemRoot%\System32` (and its subtree) | `system32` |
| `%SystemRoot%` | `system_root` |
| `%ProgramFiles%` | `program_files` |
| `%ProgramFiles(x86)%` | `program_files_x86` |
| `%ProgramData%\Microsoft` | `program_data_microsoft` |
| `%LOCALAPPDATA%\Microsoft` | `app_data_microsoft` |
| the application's install directory | `app_install` |
| the application's data directory, except its own store | `app_data` |
| a path that cannot be resolved | `unresolvable` |

Rules as implemented:

- **Deny by default** over those roots; a path outside every root is `Allowed`.
  The policy is a ceiling on authority, never a grant: an allowed decision still
  needs the operation's own checks.
- Comparison is case-insensitive and **component-aware**, so `C:\Windows10` is not
  inside `C:\Windows`. Trailing dots and spaces are trimmed, `.`/`..` are resolved
  lexically, and `\\?\C:\...` is treated as the same location as `C:\...`.
- The **most specific** matching root supplies the reason (`System32` over
  `SystemRoot`), so a denial is explainable rather than vague.
- A path that is not drive-rooted — relative, drive-less, device (`\\.\`), verbatim
  global, or UNC — is denied as `unresolvable` instead of being assumed safe. A
  share can point into a protected tree, so it is not treated as ordinary user
  space.
- Roots that cannot themselves be resolved are dropped, never allowed to match
  everything.
- Policy lives in Rust, never in the UI, and a denial always carries a reason.

Limits of this layer, by design:

- Resolution is **lexical**. 8.3 short names and reparse resolution are not
  expanded here; the paired defence is the handle-based identity check
  (`adapters/filesystem/directory.rs`), which binds the object opened to the
  object recorded.
- Non-ASCII case folding, alternate data streams, and hard links are not modelled;
  ADS and hard links inherit their containing path, which is the safe direction.
- The walker does **not** consult the policy: scanning `C:\Windows` read-only is a
  legitimate use and returns counters only.

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

Only the fourth item has any implementation: the protected-path policy denies
with a reason and is unit-tested, but no command enforces it yet, so the box stays
unchecked. The rest need commands that do not exist.

## 9. Open questions

1. Which specific operations genuinely require elevation?
2. Is a machine-wide policy override ever needed (enterprise)?
3. Should quarantine live in the user profile or a machine-wide protected folder?
4. How is the protected-path list made auditable and updatable?
5. Are network shares and removable volumes scanned, warned about, or refused?
   (Open since the B-040 review; not decided. `THREAT_MODEL.md` §11.7.)
6. Should scan roots be constrained to paths chosen through a folder picker?
