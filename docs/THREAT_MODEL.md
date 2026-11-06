# Threat Model

> **Status:** Initial draft. **No security review has been performed.** This model
> must be revisited before any destructive, privileged, or networked feature ships.
> **Last updated:** 2026-11-06

---

## 1. System under analysis

A local Windows desktop application that inspects system state and, eventually,
quarantines files and controls processes/services with user confirmation.

**Deployment:** one installer, one user session, no server.
**Components:** WebView2 renderer, Tauri IPC boundary, Rust core, Windows adapters,
SQLite store, quarantine directory, NSIS installer.

## 2. Assets

| Asset | Why it matters | Where it lives |
|---|---|---|
| User files | Deletion/corruption is irreversible | Filesystem |
| Quarantine contents | May be malware; must not be executable/leaked | `%LOCALAPPDATA%\...\quarantine` |
| Scan history | Reveals user activity and paths | SQLite |
| Logs / support bundles | May contain paths, usernames | `logs/` |
| Signing keys | Compromise enables malicious updates | CI secret store |
| User trust | The product's only real asset | — |
| System integrity | App must not weaken the machine | OS |

## 3. Trust boundaries

```text
[ OS / filesystem / registry / WMI ]  ← untrusted, fallible
        │ adapters
[ Rust core ]                         ← trusted after validation
        │ IPC (untrusted input)
[ Tauri boundary + capabilities ]     ← enforcement point
        │
[ WebView2 renderer ]                 ← assumed compromisable
        │
[ Network ]                           ← absent by design
```

| Boundary | Direction | Trust | Control |
|---|---|---|---|
| Renderer → IPC | inbound | **Untrusted** | Schema validation, capability check, policy check |
| IPC → Renderer | outbound | Trusted data, untrusted sink | DTO allowlist, no handles, size limits |
| Core → Win32 | outbound | OS is fallible/hostile | Explicit error handling, timeouts |
| Core → filesystem (write) | outbound | Hostile names/links | Identity verification, protected paths, TOCTOU handling |
| Core → DB | local | Tamperable by the user/other processes | ACLs, integrity checks, no secrets |
| Build → release | supply chain | Critical | Signing, lockfiles, SBOM, review |

## 4. Adversaries

| Adversary | Capability | Goal |
|---|---|---|
| A1 Local unprivileged user/process | Can read app data dir, modify DB/logs, craft files | Tamper with audit, read quarantine |
| A2 Malware already on the host | Can create hostile files/names/links, hook APIs | Exploit the scanner to escalate or hide |
| A3 Malicious removable media | Controls file names, directory structure, junctions | Trigger traversal bugs, path confusion |
| A4 Compromised renderer (XSS/dependency) | Can call any permitted IPC command | Reach the OS, exfiltrate data |
| A5 Network attacker | Would need egress; none today | Intercept future reputation lookups |
| A6 Supply-chain attacker | Compromised dependency or update channel | Ship code execution to users |
| A7 Social engineer targeting the user | Can make the UI show a scary finding | Get the user to approve a destructive action |

## 5. Abuse cases and mitigations

| ID | Abuse case | Impact | Mitigation | Residual risk |
|---|---|---|---|---|
| T-01 | Malicious filename injects SQL/log/HTML | Injection, spoofed UI | Parameterized queries; escape output; never `dangerouslySetInnerHTML` | Low |
| T-02 | Junction/symlink redirects a write to a protected path | System file destroyed | Do not follow reparse points for writes; verify volume + file ID | Medium until tested |
| T-03 | TOCTOU: file replaced between scan and quarantine | Wrong file quarantined/deleted | Open by handle, re-verify identity/hash, operate on handle | Medium |
| T-04 | Crafted IPC payload (huge limit, `..\` path) | DoS, path escape | Clamp limits, canonicalize, reject traversal, bound payload size | Low |
| T-05 | Compromised renderer invokes privileged command | Unauthorized system change | Capability gating + Rust-side policy + confirmation nonce | Low |
| T-06 | Quarantine file executed by user | Malware runs | Store non-executable (renamed + ACL), document, never auto-open | Low |
| T-07 | Quarantine file replaced/modified | Restore wrong content | SHA-256 on store/restore, fail closed | Low |
| T-08 | Audit log edited to hide an action | Lost accountability | Append-only + hash chain; ACLs | Medium (local admin can always win) |
| T-09 | Reparse loop / deep tree DoS | Hang or OOM | Visited-set by file ID, depth cap, cancellation | Low |
| T-10 | Fake "critical malware" finding tricks user into deleting data | Data loss | Evidence + confidence, no auto-delete, explicit consequence text | Medium |
| T-11 | Dependency compromise | Code execution | Lockfiles, review, advisory gate, signing | Medium |
| T-12 | Unsigned installer replaced in transit | Malware install | Sign + checksums + HTTPS distribution | High until signing |
| T-13 | Support bundle leaks paths/usernames | Privacy breach | Redaction by construction, preview before share | Low |

## 6. STRIDE summary

| Category | Applies to | Primary controls |
|---|---|---|
| **S**poofing | Update channel, installer, findings | Signing, checksums, rule provenance |
| **T**ampering | DB, quarantine, audit, logs | Hashes, ACLs, hash chain, integrity checks |
| **R**epudiation | User actions | Append-only audit with confirmation text |
| **I**nformation disclosure | Logs, bundles, quarantine | Redaction, ACLs, minimal collection |
| **D**enial of service | Traversal, IPC, DB | Bounds, cancellation, timeouts, paging |
| **E**levation of privilege | Renderer→OS, app→admin | Capabilities, validation, no auto-elevation |

## 7. Assumptions

1. The OS enforces user ACLs correctly.
2. WebView2 is patched by the OS/vendor.
3. The user's account is not already fully compromised (a local admin can always
   tamper with local data; we do not claim otherwise).
4. The signing key is stored outside the repository and CI logs.
5. No network egress exists in the baseline.

## 8. Out of scope

- Kernel-mode attacks and rootkits actively subverting user-mode APIs.
- Physical access.
- Multi-user/fleet scenarios.
- Protecting the user from themselves if they ignore clear warnings and confirm a
  destructive action (we minimize this through design but cannot eliminate it).

## 9. Open questions

1. Is the audit hash chain required for a single-user app? (See `AUDIT_LOGGING.md`.)
2. How do we present a quarantine item without enabling accidental execution?
3. What is the policy for scanning network shares and removable media?
4. Do we need a confirmation nonce to prevent replay of an approved action?
5. How is tampering with the local DB detected and communicated?

## 10. Review triggers

- Any new write/delete path.
- Any elevation.
- Any network egress.
- Any new capability or plugin.
- Before each public release.
